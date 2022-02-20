use clap::{Parser, Subcommand};
use std::path::Path;

mod recorder;
mod util;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    /// verbose output
    #[clap(short)]
    verbose: bool
}

// Available commands
#[derive(Subcommand, Debug)]
enum Command {
    /// Record audio
    Record {
        /// Output directory for files
        #[clap(short)]
        output_dir: String,

        /// List of inputs to ignore when recording
        #[clap(short)]
        inputs: Vec<String>,
    },

    /// List available JACK ports
    List,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Record { output_dir, inputs } => {
            if !Path::new(&output_dir).exists() {
                eprintln!("Output directory {} does not exist!", output_dir);
                return;
            }

            recorder::record(&output_dir, inputs, cli.verbose);
        },
        Command::List => recorder::listports()
    }
}
