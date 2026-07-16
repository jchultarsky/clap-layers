//! Handling sensitive data - keeping passwords out of environment and config files.
//!
//! This example demonstrates how to use `#[layered(no_env, no_file)]` to ensure
//! that sensitive fields are only provided via CLI, not from environment variables
//! or config files.

use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered, Debug)]
struct Config {
    /// Database username (can be in env/file)
    #[arg(long, default_value_t = String::from("admin"))]
    db_user: String,

    /// Database password (CLI only for security)
    /// 
    /// Note: To use `layered()` with this field, it must have a type that implements
    /// FromStr. Option<String> works because clap handles it specially during parsing.
    #[layered(no_env, no_file)]
    db_password: String,
}

fn main() {
    let cfg = Config::parse_from(std::env::args());
    
    println!("Configuration:");
    println!("  DB User: {}", cfg.db_user);
    if !cfg.db_password.is_empty() {
        println!("  DB Password: ***{}***", cfg.db_password.chars().take(3).collect::<String>());
    } else {
        println!("  DB Password: (not provided)");
    }
}
