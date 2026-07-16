use clap::Parser;
use layers::Layered;

#[derive(Parser, Layered, Debug)]
#[command(version, about = "Test")]
struct Config {
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

fn main() {
    let cfg = Config::layered().expect("Failed to load config");
    println!("{cfg:?}");
}
