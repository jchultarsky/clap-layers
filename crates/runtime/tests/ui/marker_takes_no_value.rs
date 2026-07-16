use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered)]
#[layered(env_prefix = "MYAPP")]
struct Config {
    #[layered(no_env = "true")]
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

fn main() {}
