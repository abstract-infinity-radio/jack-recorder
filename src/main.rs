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
        #[clap(short)]
        inputs: Vec<String>
    },

    /// List available JACK ports
    List,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Record { inputs } => {
            recorder::record(inputs);
        },
        Command::List => recorder::listports()
    }
}
