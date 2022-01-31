use clap::{Parser, Subcommand};

mod recorder;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

// Available commands
#[derive(Subcommand, Debug)]
enum Command {
    /// Record audio
    Record {
        /// List of inputs to ignore when recording
        inputs: Vec<String>
    },

    /// List available JACK ports
    List,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Record { inputs } => {
            record(inputs);
        },
        Command::List => recorder::listports()
    }
}

fn record(inputs: Vec<String>) {
    println!("Recording the following inputs:");

    for input in &inputs {
        println!("\t {}", input)
    }
}