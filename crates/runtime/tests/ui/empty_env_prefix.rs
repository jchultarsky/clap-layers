use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered)]
#[layered(env_prefix = "")]
struct Config {
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

fn main() {}
