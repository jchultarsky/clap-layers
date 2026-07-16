//! Keeping a credential out of the config file.
//!
//! A config file is the layer most likely to end up in version control, so
//! `#[layered(no_file)]` is the marker that matters for a secret: the field can
//! still come from the environment — the usual channel for credentials — but a
//! `db_password` key in `myapp.toml` can never populate it, even by accident.
//!
//! `#[layered(no_env, no_file)]` narrows a field to the command line alone.
//! Reach for that sparingly: a command-line argument is visible in `ps` output
//! to every other user on the machine, and is recorded in shell history, so it
//! is usually a *worse* place for a secret than the environment.
//!
//! ## Running this example
//!
//! ```bash
//! # The usual channel: read the secret from the environment, without ever
//! # writing its value into a command line or a file.
//! MYAPP_DB_PASSWORD="$DB_PASSWORD" cargo run --example sensitive_data
//!
//! # db_user is read from examples/config.toml as normal...
//! MYAPP_DB_USER=readonly cargo run --example sensitive_data
//!
//! # ...but a `db_password` key in that file is ignored, because the field
//! # opts out of the file layer.
//! ```

use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered, Debug)]
#[layered(file = "examples/config.toml", env_prefix = "MYAPP")]
struct Config {
    /// Database username; may come from any layer, config file included.
    #[arg(long, default_value_t = String::from("admin"))]
    db_user: String,

    /// Database password.
    ///
    /// `no_file` removes the config-file layer for this field only. It may
    /// still be set by `MYAPP_DB_PASSWORD`, or by `--db-password` with the
    /// caveat above.
    #[layered(no_file)]
    #[arg(long, default_value_t = String::new())]
    db_password: String,
}

fn main() {
    // `expect`/`?` would print the Debug representation, throwing away the
    // source-attributed message. Print the Display form instead.
    let cfg = Config::layered().unwrap_or_else(|e| {
        eprintln!("configuration error: {e}");
        std::process::exit(1);
    });

    println!("DB user:     {}", cfg.db_user);

    // Never print a secret, not even partially: a prefix still leaks into logs.
    // Report only whether one arrived, which is what a diagnostic needs.
    println!(
        "DB password: {}",
        if cfg.db_password.is_empty() {
            "<not set>"
        } else {
            "<set>"
        }
    );
}
