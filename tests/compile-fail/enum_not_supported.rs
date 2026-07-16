use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered)]
enum Config {
    A { port: u16 },
    B { host: String },
}

fn main() {}
