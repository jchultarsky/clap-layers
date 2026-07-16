//! Using config files for persistent configuration.
//!
//! This example demonstrates how to load configuration from a TOML file.
//!
//! ## Running this example
//!
//! First, create a `config.toml` file:
//! ```toml
//! # config.toml
//! host = "0.0.0.0"
//! port = 9000
//! ```
//!
//! Then run:
//! ```bash
//! cargo run --example config_file
//! ```

use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered, Debug)]
#[layered(file = "config.toml")]
struct Config {
    /// Host to bind to
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    host: String,

    /// Port to listen on
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

fn main() {
    let cfg = Config::layered().expect("Failed to load config");

    println!("Server configuration from file:");
    println!("  Host: {}", cfg.host);
    println!("  Port: {}", cfg.port);
}
