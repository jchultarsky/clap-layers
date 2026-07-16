use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered)]
struct Config(u16);

fn main() {}
