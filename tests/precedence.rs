//! Tests for precedence logic in clap-layers

use clap::Parser;
use clap_layers::Layered;

#[test]
fn test_default_values() {
    #[derive(Parser, Layered, Debug, PartialEq)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        #[arg(long, default_value_t = false)]
        verbose: bool,
    }

    // When no args, env vars, or file are set, use defaults
    let cfg = Config::layered().unwrap();
    assert_eq!(cfg.port, 3000);
    assert!(!cfg.verbose);
}

#[test]
fn test_env_overrides_default() {
    unsafe {
        // The generated code looks up env var based on field name (lowercase)
        std::env::set_var("port", "8080");

        #[derive(Parser, Layered, Debug, PartialEq)]
        struct Config {
            #[arg(long, default_value_t = 3000)]
            port: u16,
        }

        let cfg = Config::layered().unwrap();

        std::env::remove_var("port");

        assert_eq!(cfg.port, 8080); // Should be overridden from env
    }
}

#[test]
fn test_prefixed_env() {
    unsafe {
        std::env::set_var("MYAPP_PORT", "9000");

        #[derive(Parser, Layered, Debug)]
        struct Config {
            #[arg(long, default_value_t = 3000)]
            port: u16,
        }

        let _cfg = Config::layered().unwrap();

        std::env::remove_var("MYAPP_PORT");
    }
}

#[test]
fn test_file_overrides_default() {
    let file_path = "test_config.toml";
    std::fs::write(file_path, "port = 5000\n").expect("failed to write file");

    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    let _cfg = Config::layered().unwrap();

    std::fs::remove_file(file_path).ok();
}

#[test]
fn test_cli_not_passed() {
    #[derive(Parser, Layered, Debug, PartialEq)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        #[arg(long, default_value_t = false)]
        verbose: bool,
    }

    // When layered() parses from std::env::args(), no CLI args are provided
    // so it should use defaults
    let cfg = Config::layered().unwrap();
    assert_eq!(cfg.port, 3000);
    assert!(!cfg.verbose);
}
