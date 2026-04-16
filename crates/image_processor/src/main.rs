use clap::Parser;
use image_processor::{Cli, run};

fn main() {
    env_logger::init();

    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
