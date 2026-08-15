mod agent;
mod config;
mod llm;
mod memory;
mod server;
mod sim;
mod state;
mod town;

use std::sync::{Arc, RwLock};

use clap::Parser;
use colored::Colorize;

use config::{Args, Config};
use state::SharedState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = Config::from_args(&args);

    let shared = Arc::new(RwLock::new(SharedState::new()));

    let sim_shared = shared.clone();
    let sim_config = config.clone();
    std::thread::Builder::new()
        .name("sim".to_string())
        .spawn(move || {
            if let Err(e) = sim::run(&sim_config, &sim_shared) {
                eprintln!("{}", format!("simulation error: {e}").red());
            }
        })?;

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(server::run(&config, shared))?;
    Ok(())
}
