use clap::Parser;
use clap_layers::Layered;

// `MY-APP_PORT` is not a settable environment variable name, so the layer would
// silently never fire.
#[derive(Parser, Layered)]
#[layered(env_prefix = "my-app")]
struct Config {
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

fn main() {}
