use clap::Parser;
use clap_layers::Layered;

// Both fields are wrong. The derive must report *both* in one build rather
// than stopping at the first.
#[derive(Parser, Layered)]
#[layered(env_prefix = "MYAPP")]
struct Config {
    #[layered(no_envv)]
    #[arg(long, default_value_t = 1)]
    first: u16,

    #[layered(no_cli)]
    #[arg(long, default_value_t = 2)]
    second: u16,
}

fn main() {}
