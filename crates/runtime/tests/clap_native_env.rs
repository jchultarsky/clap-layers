//! clap's own `#[arg(env = "...")]` must compose with our layers.
//!
//! This is the one test that mutates the real process environment: clap reads
//! `getenv` itself, so it cannot be injected. It lives in its own test binary so
//! no other test thread can observe the mutation — `set_var` is `unsafe` in
//! edition 2024 precisely because it races with concurrent `getenv`.

use clap::Parser;
use clap_layers::Layered;

mod support;
use support::{TempToml, no_env};

/// clap's own `#[arg(env = ...)]` must keep beating the file layer: it is a real
/// user-supplied value, not a default.
#[test]
fn claps_native_env_support_beats_the_file_layer() {
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/matrix_clapenv.toml")]
    struct Config {
        #[arg(long, env = "MX_CLAP_NATIVE_PORT", default_value_t = 3000)]
        port: u16,
    }

    let _f = TempToml::new("matrix_clapenv.toml", "port = 5000\n");

    // clap reads the process environment itself for `#[arg(env)]`, so this case
    // has to set a real variable. The name is unique to this test.
    unsafe { std::env::set_var("MX_CLAP_NATIVE_PORT", "7777") };
    let cfg = Config::layered_from(["t"], &no_env()).unwrap();
    unsafe { std::env::remove_var("MX_CLAP_NATIVE_PORT") };

    assert_eq!(
        cfg.port, 7777,
        "a value clap read from the environment must beat the config file"
    );
}
