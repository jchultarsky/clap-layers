//! Dynamic values - fields that always come from environment or file.
//!
//! This example shows how to use `#[layered(no_cli)]` for fields that should
//! never be set via command-line arguments (e.g., auto-generated IDs, timestamps).

use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered, Debug)]
struct Config {
    /// Environment-specific identifier
    #[arg(long, default_value_t = String::from("unknown"))]
    environment: String,

    /// Instance ID (never set via CLI - generated or read)
    #[layered(no_cli)]
    instance_id: String,
}

fn main() {
    let cfg = Config::parse_from(std::env::args());

    println!("Configuration:");
    println!("  Environment: {}", cfg.environment);
    println!("  Instance ID: {}", cfg.instance_id);
}
