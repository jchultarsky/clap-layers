use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered)]
struct Config {
    // Hidden from the CLI, with no env_prefix and no file: this field could
    // only ever hold Default::default().
    #[layered(no_cli)]
    #[arg(skip)]
    unreachable: String,
}

fn main() {}
