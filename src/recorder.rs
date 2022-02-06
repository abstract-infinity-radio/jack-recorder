use jack;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

const JACK_CLIENT_NAME: &str = "Art Infinity Radio";

pub fn listports() {
    // connect to JACK
    let client = match jack::Client::new(JACK_CLIENT_NAME, jack::ClientOptions::NO_START_SERVER) {
        Ok((client, _status)) => client,
        Err(err) => {
            eprintln!("Jack server not running?! {}", err);
            std::process::exit(-1);
        }
    };

    // list output ports
    let port_list = client.ports(None, None, jack::PortFlags::IS_OUTPUT);
    println!("Available ports to record from:");
    for val in port_list.iter() {
        println!("{}", val);
    }
}

pub fn record(ignored_inputs: Vec<String>) {
    let mut ports_to_record: Vec<String> = Vec::new();

    let client = match jack::Client::new(JACK_CLIENT_NAME, jack::ClientOptions::NO_START_SERVER) {
        Ok((client, _status)) => client,
        Err(err) => {
            eprintln!("Jack server not running?! {}", err);
            std::process::exit(-1);
        }
    };

    // get the list of output ports to record from
    let port_list = client.ports(None, None, jack::PortFlags::IS_OUTPUT);
    for val in port_list.iter() {
        if ignored_inputs.contains(val) {
            continue;
        }
        ports_to_record.push(val.clone());
    }

    println!("Recording the following inputs:");
    for port_name in &ports_to_record {
        println!("\t{}", port_name)
    }

    if ports_to_record.len() == 0 {
        eprintln!("No ports to record!");
        return;
    }

    println!("Press CTRL-C to stop recording...");

    // setup CTRL-C handler
    let should_stop_flag = Arc::new(AtomicBool::new(false));
    let should_stop = should_stop_flag.clone();
    ctrlc::set_handler(move || {
        println!("CTRL-C detected!");
        should_stop_flag.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // setup recording
    let mut recording_port_tuples: Vec<(&str, jack::Port<jack::AudioIn>)> = Vec::new();
    for port_name in &ports_to_record {
        match record_port(port_name, &client) {
            Ok(port) => {
                let port_name_copy = string_to_static_str(String::from(port_name));
                recording_port_tuples.push((port_name_copy, port));
            }
            Err(e) => {
                eprintln!("Could not create recording port for {}: {}", port_name, e);
                continue;
            }
        }
    }

    // connect input ports to recording ports
    let should_stop_cb =should_stop.clone();
    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| {
        println!("Processing {} frames", ps.n_frames());
        if should_stop_cb.load(Ordering::Relaxed) {
            // receiving callbacks after call to `client.deactivate()`
            // causes panics in rust lib so this effectively stops this processing
            return jack::Control::Quit;
        }
        for _ in &recording_port_tuples {}
        return jack::Control::Continue;
    };
    let process_loop = jack::ClosureProcessHandler::new(process_callback);
    let client = match client.activate_async((), process_loop) {
        Ok(client) => client,
        Err(e) => panic!("Could not activate JACK client! {}", e),
    };

    // wait until it is finished
    while !should_stop.load(Ordering::Relaxed) {
        sleep(Duration::from_millis(100));
    }
    
    sleep(Duration::from_millis(100));
    // drop(client);
    match client.deactivate() {
        Ok(_) => (),
        Err(e) => eprintln!("Error while deactivating JACK client: {}", e),
    }
    // println!("Deactivated");
    // sleep in order to let the JACK cleanup resources...
    // TODO: wait for wav recording to stop.
}

fn record_port(
    port_name: &str,
    jack_client: &jack::Client,
) -> Result<jack::Port<jack::AudioIn>, jack::Error> {
    println!("Recoding port... {}", port_name);

    let port_prefix = "jack_recorder:";
    let recording_port_name = format!("{}{}", port_prefix, port_name);

    // create a recording port
    match jack_client.register_port(&recording_port_name, jack::AudioIn::default()) {
        Ok(port) => Ok(port),
        Err(e) => Err(e),
    }

    // connect capture port with recording port created above
}

// This is required in order to provide a list of port names to JACK callback
fn string_to_static_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
