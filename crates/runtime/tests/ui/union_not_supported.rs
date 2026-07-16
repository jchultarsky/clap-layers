use clap_layers::Layered;

#[derive(Layered)]
union Config {
    port: u16,
}

fn main() {}
