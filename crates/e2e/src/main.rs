//! A minimal application driven by [`Layered::layered`], for end-to-end tests.
//!
//! Deliberately has no config file, so its behaviour does not depend on the
//! working directory the test harness happens to use. The file layer is covered
//! in-process by the runtime crate's own suites.

use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered, Debug)]
#[command(name = "clap-layers-e2e", version = "1.2.3")]
#[layered(env_prefix = "MYAPP")]
struct Config {
    /// Host to bind to
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    host: String,

    /// Port to listen on
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Verbosity
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    // The pattern the docs recommend: `layered()` handles CLI errors and
    // `--help` itself; print the Display form of anything else, because `?` in
    // `main` would print the Debug representation instead.
    let cfg = Config::layered().unwrap_or_else(|e| {
        eprintln!("configuration error: {e}");
        std::process::exit(1);
    });

    println!("host={}", cfg.host);
    println!("port={}", cfg.port);
    println!("verbose={}", cfg.verbose);
}
