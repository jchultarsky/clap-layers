use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered)]
#[layered(env_prefix = "MYAPP")]
struct Config {
    // A typo here used to be silently ignored, quietly re-enabling the
    // environment layer for a field meant to opt out of it.
    #[layered(no_envv)]
    #[arg(long, default_value_t = String::from(""))]
    secret: String,
}

fn main() {}
