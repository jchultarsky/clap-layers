//! Complete layered configuration example.
//!
//! This example demonstrates all features working together:
//! - CLI arguments (highest priority)
//! - Environment variables (second priority)
//! - Config file (third priority)
//! - Default values (lowest priority)
//!
//! ## Precedence Order
//!
//! When a value is requested, the system checks in this order:
//! 1. CLI argument (e.g., `--port 8080`)
//! 2. Environment variable (e.g., `MYAPP_PORT=8080`)
//! 3. Config file value (from `config.toml`)
//! 4. Default value (in struct definition)
//!
//! ## Running this example
//!
//! 1. Create a config file at `examples/config.toml`:
//! ```toml
//! database_url = "postgres://localhost:5432/dev"
//! cache_ttl = 60
//! ```
//!
//! 2. Run with different configurations:
//! ```bash
//! # Use all defaults
//! cargo run --example complete_example
//!
//! # Override via environment variable
//! MYAPP_DATABASE_URL="postgres://prod:5432/prod" cargo run --example complete_example
//!
//! # Override via CLI (highest priority)
//! cargo run --example complete_example -- --database-url "postgres://cli:5432/cli"
//!
//! # Combine: file has values, env overrides some, CLI overrides all
//! MYAPP_DEBUG=false cargo run --example complete_example -- --debug --cache-ttl 120
//! ```

use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered, Debug)]
#[command(
    version = "1.0.0",
    about = "Complete layered configuration example",
    long_about = None
)]
#[layered(file = "config.toml", env_prefix = "MYAPP")]
struct Config {
    /// Database connection URL
    #[arg(long, default_value_t = String::from("sqlite://localhost:3000/db"))]
    database_url: String,

    /// Redis cache URL
    #[arg(long, default_value_t = String::from("redis://127.0.0.1:6379/0"))]
    redis_url: String,

    /// Cache time-to-live in seconds
    #[arg(long, default_value_t = 300)]
    cache_ttl: u64,

    /// Whether to enable debug mode
    #[arg(long, default_value_t = false)]
    debug: bool,
}

fn main() {
    let cfg = Config::layered().expect("Failed to load configuration");

    println!("=== Application Configuration ===\n");

    println!("Database: {}", cfg.database_url);
    println!("Redis:    {}", cfg.redis_url);
    println!("Cache TTL: {}s ({}m)", cfg.cache_ttl, cfg.cache_ttl / 60);
    println!("Debug:     {}", cfg.debug);
}
