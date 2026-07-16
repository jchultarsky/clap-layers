use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered)]
#[layered(env_prefix = "MYAPP")]
struct Config {
    #[layered(no_cli)]
    #[arg(long, default_value_t = String::from("x"))]
    instance_id: String,
}

fn main() {}
