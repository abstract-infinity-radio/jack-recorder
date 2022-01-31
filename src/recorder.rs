use jack;

pub fn listports() {
    // connect to JACK
    // let (client, _status) = jack::Client::new("Air", jack::ClientOptions::NO_START_SERVER).expect("Jack server not running?");
    let client = match jack::Client::new("Air", jack::ClientOptions::NO_START_SERVER) {
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
