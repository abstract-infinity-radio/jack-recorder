use clap::{Parser, Subcommand};
use log::LevelFilter;
use log::{info, warn};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    /// verbose output
    #[clap(short)]
    verbose: bool,
}

// Available commands
#[derive(Subcommand, Debug)]
enum Command {
    /// Record audio
    Record {
        /// Output directory for files
        #[clap(short)]
        output_dir: String,

        /// List of inputs to record
        #[clap(short)]
        inputs: Vec<String>,
    },

    /// List available JACK ports
    List,
}

fn main() {
    simple_logging::log_to_stderr(LevelFilter::Debug);
    let cli = Cli::parse();

    match cli.command {
        Command::Record { output_dir, inputs } => {
            if !Path::new(&output_dir).exists() {
                warn!("Output directory {} does not exist!", output_dir);
                return;
            }
            let should_stop = Arc::new(AtomicBool::new(false));
            {
                let should_stop = should_stop.clone();
                ctrlc::set_handler(move || {
                    info!("CTRL-C detected!");
                    should_stop.store(true, Ordering::Relaxed);
                })
                .expect("Error setting Ctrl-C handler");
            }
            // setup CTRL-C handler
            info!("Press CTRL-C to stop recording...");

            jack_recorder::record(&output_dir, inputs, cli.verbose, should_stop);
        }
        Command::List => match jack_recorder::listports() {
            Ok(ports) => {
                for val in ports.iter() {
                    info!("{}", val);
                }
            }
            Err(e) => {
                warn!("{}", e);
            }
        },
    }
}
