//! Environment variable precedence over config file.
//!
//! This example shows how environment variables override values from config files.
//!
//! ## Running this example
//!
//! 1. Create a `config.toml`:
//! ```toml
//! port = 3000
//! ```
//!
//! 2. Run with different priorities:
//! ```bash
//! # Config file value (3000)
//! cargo run --example precedence_demo
//!
//! # Environment variable overrides config file (8080 > 3000)
//! MYAPP_PORT=8080 cargo run --example precedence_demo
//!
//! # CLI overrides everything
//! cargo run --example precedence_demo -- --port 9000
//! ```

use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered, Debug)]
#[layered(file = "config.toml", env_prefix = "MYAPP")]
struct Config {
    /// Port to listen on
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

fn main() {
    let cfg = Config::layered().expect("Failed to load config");
    
    println!("Final configuration:");
    println!("  Port: {}", cfg.port);
    println!("\nRemember the precedence order:");
    println!("  CLI flag > env var > config file > default");
}
