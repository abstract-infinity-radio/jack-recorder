use chrono::{Datelike, Timelike, Utc};
use crossbeam_channel::{unbounded, RecvTimeoutError};
use jack;
use log::{error, info, warn};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::fs::create_dir_all;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use zip::write::FileOptions;

const JACK_CLIENT_NAME: &str = "Art Infinity Radio";

#[derive(Debug)]
struct RecordingPortContainer {
    src_port_name: std::string::String,
    dst_port_name: std::string::String,
    port: jack::Port<jack::AudioIn>,
}

#[derive(Debug)]
struct SampleContainer {
    src_port_name: String,
    samples: Vec<f32>,
}

impl SampleContainer {
    pub fn new(port_name: &str, data: &[f32]) -> Self {
        SampleContainer {
            src_port_name: String::from(port_name),
            samples: Vec::from(data),
        }
    }
}

pub fn listports() -> Result<Vec<String>, String> {
    // connect to JACK
    let client = match jack::Client::new(JACK_CLIENT_NAME, jack::ClientOptions::NO_START_SERVER) {
        Ok((client, _status)) => client,
        Err(err) => {
            let msg = format!("Jack server not running?! {}", err);
            warn!("{}", msg);
            return Err(msg);
        }
    };

    // list output ports
    let mut ports: Vec<String> = vec![];
    let port_list = client.ports(None, None, jack::PortFlags::IS_OUTPUT);
    info!("Available ports to record from:");
    for val in port_list.iter() {
        ports.push(format!("{}", val));
    }
    Ok(ports)
}

pub fn record(
    output_dir: &String,
    port_selection: Vec<String>,
    verbose: bool,
    should_stop: Arc<AtomicBool>,
) {
    let mut ports_to_record: Vec<String> = Vec::new();

    // create JACK client
    let client = match jack::Client::new(JACK_CLIENT_NAME, jack::ClientOptions::NO_START_SERVER) {
        Ok((client, _status)) => client,
        Err(err) => {
            warn!("Jack server not running?! {}", err);
            std::process::exit(-1);
        }
    };

    // get the list of output ports to record from
    let port_list = client.ports(None, None, jack::PortFlags::IS_OUTPUT);
    for val in port_list.iter() {
        if port_selection.len() == 0 || port_selection.contains(val) {
            ports_to_record.push(val.clone());
        }
    }

    if verbose {
        info!("Recording the following inputs:");
        for port_name in &ports_to_record {
            info!("\t{}", port_name)
        }
    }

    if ports_to_record.len() == 0 {
        warn!("No ports to record!");
        return;
    }

    // setup recording ports
    let wawfile_specification = hound::WavSpec {
        channels: 1,
        sample_rate: client.sample_rate() as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut recording_ports: Vec<RecordingPortContainer> = Vec::new();
    for port_name in &ports_to_record {
        match setup_recording_port(port_name, &client) {
            Ok((recording_port_name, port)) => {
                let r = RecordingPortContainer {
                    src_port_name: String::from(port_name),
                    dst_port_name: String::from(recording_port_name),
                    port: port,
                };
                recording_ports.push(r);
            }
            Err(e) => {
                warn!("Could not create recording port for {}: {}", port_name, e);
                continue;
            }
        }
    }

    // copy of ports vector to be shared around (shadowed variable)
    let recording_ports: Arc<Vec<RecordingPortContainer>> = Arc::from(recording_ports);

    // setup cross-thread communication with crossbeam
    let (tx, rx) = unbounded::<SampleContainer>();

    let write_thread;
    {
        let recording_ports = recording_ports.clone();
        let should_stop = should_stop.clone();
        let output_dir = String::from(output_dir);
        write_thread = std::thread::spawn(move || {
            let now = Utc::now();
            let timestamp = format!(
                "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}",
                now.year(),
                now.month(),
                now.day(),
                now.hour(),
                now.minute(),
                now.second()
            );
            // create timestamp directory for each recording "session"
            let output_dir = Path::new(&output_dir).join(Path::new(&timestamp));
            match create_dir_all(&output_dir) {
                Ok(ok) => ok,
                Err(e) => {
                    error!("Could not create target directory {:?} - {}", output_dir, e);
                    return;
                }
            };
            // setup wav files dictionary
            let mut port_files = HashMap::new();
            for i in 0..recording_ports.len() {
                let rec_info = &recording_ports[i];
                let filename = Path::new(&output_dir)
                    .join(Path::new(&format!(
                        "{}-{}-{}.wav",
                        timestamp,
                        i,
                        rec_info.src_port_name.replace(is_unsafe_char, "_")
                    )))
                    .to_str()
                    .unwrap()
                    .to_owned();
                port_files.insert(
                    rec_info.src_port_name.clone(),
                    hound::WavWriter::create(filename, wawfile_specification).unwrap(),
                );
            }

            let mut counter = 0;
            loop {
                match rx.recv_timeout(Duration::from_millis(20)) {
                    Ok(_data) => {
                        match port_files.get_mut(&_data.src_port_name) {
                            Some(w) => {
                                for sample in _data.samples {
                                    w.write_sample(sample).unwrap();
                                }
                            }
                            None => warn!("Writer not present!?!"),
                        };
                    }
                    Err(e) => {
                        if e != RecvTimeoutError::Timeout {
                            warn!("{}", e)
                        }
                    }
                };
                if counter == 0 && verbose {
                    info!("Write queue length: {}", rx.len());
                }
                if should_stop.load(Ordering::Relaxed) {
                    if rx.len() == 0 {
                        break;
                    }
                    info!("Writing data to disk...");
                }

                counter = (counter + 1) % 1000;
            }

            // finalize each WAV writer
            for (key, entry) in port_files.drain() {
                match entry.finalize() {
                    Ok(_) => info!("Finalized writing {}", key),
                    Err(e) => info!("Error when finalizing WAV file for port {}: {}", key, e),
                }
            }
            info!("Writer thread finished!");

            info!("Creating a ZIP archive");
            match compress_files_in_directory(output_dir) {
                Ok(()) => info!("Done"),
                Err(error) => error!("Could not create ZIP archive: {}", error),
            }
        });
    }

    // setup JACK process callback
    let should_stop_cb = should_stop.clone();
    let process_callback = {
        let recording_ports = recording_ports.clone();
        move |_: &jack::Client, ps: &jack::ProcessScope| {
            if should_stop_cb.load(Ordering::Relaxed) {
                // receiving callbacks after call to `client.deactivate()`
                // causes panics in rust lib so this effectively stops this processing
                return jack::Control::Quit;
            }
            for i in 0..recording_ports.len() {
                let rec_info = &recording_ports[i];
                let input_slice = rec_info.port.as_slice(ps);

                let data = SampleContainer::new(&rec_info.src_port_name, input_slice);
                match tx.send(data) {
                    Ok(_) => (),
                    Err(e) => warn!("Error sending data to write thread: {}", e),
                };
            }
            return jack::Control::Continue;
        }
    };
    let process_loop = jack::ClosureProcessHandler::new(process_callback);
    // activate JACK client (starts async processing loop)
    let client = match client.activate_async((), process_loop) {
        Ok(client) => client,
        Err(e) => panic!("Could not activate JACK client! {}", e),
    };

    // connect input ports to recording ports
    for i in 0..recording_ports.len() {
        let rec_info = &recording_ports[i];
        info!(
            "Connecting port {} to {}",
            rec_info.src_port_name, rec_info.src_port_name
        );
        match client.as_client().connect_ports_by_name(
            rec_info.src_port_name.as_ref(),
            rec_info.dst_port_name.as_ref(),
        ) {
            Ok(_) => (),
            Err(e) => warn!(
                "Could not connect ports {} - {}: {}",
                rec_info.src_port_name, rec_info.dst_port_name, e
            ),
        }
    }

    // wait until Ctrl-C is detected
    while !should_stop.load(Ordering::Relaxed) {
        sleep(Duration::from_millis(100));
    }
    // wait a little with the deactivation in order to avoid
    // a bug in JACK
    sleep(Duration::from_millis(500));

    match client.deactivate() {
        Ok(_) => (),
        Err(e) => warn!("Error while deactivating JACK client: {}", e),
    }
    info!("Waiting for write thread to finish...");
    match write_thread.join() {
        Ok(_) => (),
        Err(_) => warn!("Error while waiting for thread to finish"),
    };

    info!("Finished recording");
}

fn setup_recording_port(
    port_name: &str,
    jack_client: &jack::Client,
) -> Result<(String, jack::Port<jack::AudioIn>), jack::Error> {
    info!("Recording port... {}", port_name);

    // create a recording port
    match jack_client.register_port(&port_name, jack::AudioIn::default()) {
        Ok(port) => Ok((format!("{}:{}", JACK_CLIENT_NAME, port_name), port)),
        Err(e) => Err(e),
    }
}

fn is_unsafe_char(x: char) -> bool {
    if (x >= 'a' && x <= 'z') || (x >= 'A' && x <= 'Z') || (x >= '0' && x <= '9') {
        return false;
    }
    return true;
}

fn compress_files_in_directory(target_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let zip_options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);
    let file = File::create(Path::new(&target_dir).join("bundle.zip"))?;
    let mut zip = zip::ZipWriter::new(file);
    let mut buffer = vec![0u8; 10 * 1024 * 1024];

    let paths = fs::read_dir(target_dir.clone())?;
    for path in paths {
        // println!("Name: {:?}", path.unwrap().path());
        let path = path.unwrap().path();
        if path.to_str().unwrap().ends_with(".zip") {
            continue;
        }
        let filename = path.file_name().unwrap().to_str().unwrap();
        match zip.start_file(filename, zip_options) {
            Ok(_) => {
                let mut input_file = File::open(path)?;
                loop {
                    let count_read = input_file.read(&mut buffer)?;
                    if count_read == 0 {
                        break;
                    }

                    zip.write(&buffer[0..count_read])?;
                }
            }
            Err(err) => error!("Could not add {:?} to zip archive: {}", path, err),
        };
    }
    zip.finish()?;
    Ok(())
}
