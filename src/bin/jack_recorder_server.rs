use chrono::{DateTime, Utc};
use clap::Parser;
use log::{debug, error, info, LevelFilter};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use warp::reply::Response;
use warp::Filter;
use warp::Reply;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// verbose output
    #[clap(short)]
    verbose: bool,
    /// output directory
    #[clap(short)]
    output_dir: String,

    #[clap(short)]
    port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct RecordingStatus {
    pub is_recording: bool,
    pub start_time: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct StartRecording {
    pub ports: Vec<String>,
}

#[tokio::main]
async fn main() {
    let recording_start_time: Arc<Mutex<Option<DateTime<Utc>>>> = Arc::new(Mutex::new(None));
    let recording_thread_handle = Arc::new(Mutex::new(Option::None));
    let is_recording = Arc::new(AtomicBool::new(false));
    let should_stop = Arc::new(AtomicBool::new(false));

    simple_logging::log_to_stderr(LevelFilter::Debug);
    let cli = Cli::parse();

    let list = warp::get().and(warp::path("list").map(|| -> Response {
        let mut ports: Vec<String> = vec![];
        match jack_recorder::listports() {
            Ok(port_names) => {
                for p in port_names.iter() {
                    ports.push(format!("{}", p));
                }
                warp::reply::json(&ports).into_response()
            }
            Err(e) => warp::reply::with_status(e, warp::http::StatusCode::INTERNAL_SERVER_ERROR)
                .into_response(),
        }
    }));

    let start_recording = {
        let output_dir = cli.output_dir.clone();
        let should_stop = should_stop.clone();
        let recording_start_time = recording_start_time.clone();
        let recording_thread_handle = recording_thread_handle.clone();
        let is_recording = is_recording.clone();
        warp::post()
            .and(warp::path("start_recording"))
            .and(warp::body::json())
            .map(move |start_recording: StartRecording| {
                let should_stop = should_stop.clone();
                let output_dir = output_dir.clone();
                should_stop.store(false, Ordering::Relaxed);
                if is_recording.load(Ordering::Relaxed) == false {
                    let join_handle = std::thread::spawn(move || {
                        jack_recorder::record(
                            &String::from(output_dir),
                            start_recording.ports,
                            cli.verbose,
                            should_stop,
                        );
                    });
                    is_recording.store(true, Ordering::Relaxed);
                    let _ = recording_thread_handle.lock().unwrap().insert(join_handle);
                    let _ = recording_start_time.lock().unwrap().insert(Utc::now());
                    warp::reply::with_status("OK", warp::http::StatusCode::OK).into_response()
                } else {
                    warp::reply::with_status(
                        "Already recording...",
                        warp::http::StatusCode::BAD_REQUEST,
                    )
                    .into_response()
                }
            })
    };

    let stop_recording = {
        let should_stop = should_stop.clone();
        let is_recording = is_recording.clone();
        let recording_thread_handle = recording_thread_handle.clone();
        warp::post().and(warp::path("stop_recording").map(move || {
            if is_recording.load(Ordering::Relaxed) == true {
                should_stop.store(true, Ordering::Relaxed);
                let mut maybe_thread_handle = recording_thread_handle.lock().unwrap();
                match maybe_thread_handle.take() {
                    Some(join_handle) => match join_handle.join() {
                        Ok(()) => debug!("Recording thread joined"),
                        Err(e) => error!("Could not join recording thread: {:?}", e),
                    },
                    None => error!("Expected to join recording thread but handle not present!"),
                };
                is_recording.store(false, Ordering::Relaxed);
                warp::reply::with_status("OK", warp::http::StatusCode::OK).into_response()
            } else {
                warp::reply::with_status("Not running", warp::http::StatusCode::BAD_REQUEST)
                    .into_response()
            }
        }))
    };

    let status = {
        let is_recording = is_recording.clone();
        warp::get().and(warp::path("status").map(move || {
            let is_recording = is_recording.load(Ordering::Relaxed);
            let start_time = match is_recording {
                true => Some(recording_start_time.lock().unwrap().unwrap()),
                _ => None,
            };
            let status = RecordingStatus {
                is_recording: is_recording,
                start_time: start_time,
            };
            warp::reply::json(&status)
        }))
    };

    let routes = list.or(start_recording).or(stop_recording).or(status);
    let address: SocketAddr = format!("127.0.0.1:{}", cli.port).parse().unwrap();
    info!("Starting server...");
    // warp::serve(routes).run(address).await;
    let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(address, async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c");
    });

    server.await;

    if is_recording.load(Ordering::Relaxed) == true {
        info!("Server shutdown while recording in progress!");
        should_stop.store(true, Ordering::Relaxed);

        let mut maybe_thread_handle = recording_thread_handle.lock().unwrap();
        match maybe_thread_handle.take() {
            Some(join_handle) => match join_handle.join() {
                Ok(()) => debug!("Recording thread joined"),
                Err(e) => error!("Could not join recording thread: {:?}", e),
            },
            None => (),
        };
    }
}
