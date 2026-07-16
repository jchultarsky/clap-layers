//! Simple example demonstrating the Layered derive with clap.
//!
//! This example shows how to use `clap_layers` for layered configuration
//! loading from CLI, environment variables, and config files.
//!
//! ## Precedence Order
//!
//! Values are loaded in this order (highest to lowest priority):
//!
//! 1. **CLI flags** - `--port 8080`
//! 2. **Environment variables** - `MYAPP_PORT=8080`
//! 3. **Config file** - `config.toml`
//! 4. **Built-in defaults** - `default_value_t = 3000`

use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered, Debug)]
#[command(version, about = "Test application with layered config")]
#[layered(file = "config.toml", env_prefix = "MYAPP")]
struct Config {
    /// Port to listen on
    #[arg(long, short, default_value_t = 3000)]
    port: u16,

    /// Verbosity level
    #[arg(long, short, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let cfg = Config::layered().expect("Failed to load config");
    println!("{cfg:?}");
}
