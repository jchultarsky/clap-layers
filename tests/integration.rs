//! Integration tests for clap-layers v0.1

use clap::Parser;
use clap_layers::Layered;

#[test]
fn test_example_from_readme() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        /// Port to listen on
        #[arg(long, default_value_t = 3000)]
        port: u16,

        /// Verbosity
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    }

    // Test with defaults
    let cfg = Config::parse_from(["test"]);
    assert_eq!(cfg.port, 3000);
    assert!(!cfg.verbose);

    // Test with CLI override
    let cfg = Config::parse_from(["test", "--port", "8080", "-v"]);
    assert_eq!(cfg.port, 8080);
    assert!(cfg.verbose);
}

#[test]
fn test_no_cli_field_attribute() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        /// Admin password - not exposed as CLI flag
        #[layered(no_cli)]
        #[arg(default_value = "")]
        admin_password: String,
    }

    let cfg = Config::parse_from(["test", "--port", "8080"]);
    assert_eq!(cfg.port, 8080);
}

#[test]
fn test_no_env_field_attribute() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        /// Sensitive value - not read from env
        #[layered(no_env)]
        #[arg(default_value = "")]
        api_key: String,
    }

    unsafe {
        std::env::set_var("MYAPP_API_KEY", "exposed");
    }

    // The field should still exist in the parsed struct with its default (empty string)
    let cfg = Config::parse_from(["test"]);
    assert_eq!(cfg.port, 3000);
    assert_eq!(cfg.api_key, ""); // Default for String

    unsafe { std::env::remove_var("MYAPP_API_KEY") };
}

#[test]
fn test_no_file_field_attribute() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        /// Dynamic value - not in config file
        #[layered(no_file)]
        #[arg(default_value = "")]
        instance_id: String,
    }

    // Should parse without error even with no_file attribute
    let cfg = Config::parse_from(["test"]);
    assert_eq!(cfg.port, 3000);
}
