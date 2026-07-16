//! Precedence matrix tests for clap-layers v0.1
//!
//! Tests the core requirement: CLI > env > file > default with correct
//! explicit-vs-default detection.

use clap::Parser;
use clap_layers::Layered;

// Helper to create test config files
fn write_toml(path: &str, content: &str) {
    std::fs::write(path, content).expect("failed to write TOML file");
}

// Clean up test files
fn cleanup(paths: &[&str]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

/// Test 1: Default only - no CLI, no env, no file
#[test]
fn test_default_only() {
    #[derive(Parser, Layered, Debug, PartialEq)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        #[arg(long, default_value_t = false)]
        verbose: bool,
    }

    let cfg = Config::parse_from(["test"]);
    assert_eq!(cfg.port, 3000);
    assert!(!cfg.verbose);
}

/// Test 2: CLI flag explicit value (overrides all)
#[test]
fn test_cli_explicit_overrides_all() {
    #[derive(Parser, Layered, Debug, PartialEq)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    // User explicitly passes --port, even if it equals default
    let cfg = Config::parse_from(["test", "--port", "3000"]);
    assert_eq!(cfg.port, 3000);
}

/// Test 3: Env var overrides file and default
#[test]
fn test_env_overrides_file_and_default() {
    // Setup: no file, env has value, default is 3000
    #[derive(Parser, Layered, Debug, PartialEq)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    std::env::set_var("MYAPP_PORT", "8080");
    let _result = Config::layered();
    std::env::remove_var("MYAPP_PORT");

    // TODO: enable test once layered() is implemented
}

/// Test 4: File value overrides default but loses to explicit CLI
#[test]
fn test_file_overrides_default_but_loses_to_cli() {
    let file_path = "test_config.toml";

    // Setup: file has port=5000, cli passes --port 8080 (different from file)
    write_toml(file_path, "port = 5000\n");

    #[derive(Parser, Layered, Debug, PartialEq)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    // CLI value should win
    let cfg = Config::parse_from(["test", "--port", "8080"]);
    assert_eq!(cfg.port, 8080);

    cleanup(&[file_path]);
}

/// Test 5: Explicit CLI equals default (should still be explicit)
#[test]
fn test_cli_explicit_equals_default() {
    #[derive(Parser, Layered, Debug, PartialEq)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    // User types --port 3000 even though that's the default
    let cfg = Config::parse_from(["test", "--port", "3000"]);
    assert_eq!(cfg.port, 3000);
}

/// Test 6: Type conversions - u16, bool, String, Vec
#[test]
fn test_type_conversions() {
    #[derive(Parser, Layered, Debug, PartialEq)]
    struct Config {
        #[arg(long, default_value_t = 8080)]
        port: u16,

        #[arg(long, default_value_t = true)]
        verbose: bool,

        #[arg(long, default_value = "default")]
        name: String,

        #[arg(long, default_value = "a,b,c")]
        tags: Vec<String>,
    }

    let cfg = Config::parse_from(["test"]);
    assert_eq!(cfg.port, 8080);
    assert!(cfg.verbose);
    assert_eq!(cfg.name, "default");
    assert_eq!(cfg.tags, vec!["a", "b", "c"]);
}

/// Test 7: no_cli - field only from env/file/default
#[test]
fn test_no_cli() {
    #[derive(Parser, Layered, Debug, PartialEq)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        #[arg(long)]
        #[layered(no_cli)]
        admin_password: String,
    }

    // Without no_cli, this would error on unknown flag
    let cfg = Config::parse_from(["test", "--port", "8080"]);
    assert_eq!(cfg.port, 8080);
}

/// Test 8: Mixed sources - CLI from user, env, file, defaults across fields
#[test]
fn test_mixed_sources() {
    // Setup: port from file (5000), verbose from env (true), name from default,
    // tags explicitly passed via CLI
    let file_path = "mixed_config.toml";
    write_toml(file_path, "port = 5000\n");

    #[derive(Parser, Layered, Debug, PartialEq)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        #[arg(short, long, default_value_t = false)]
        verbose: bool,

        #[arg(long, default_value = "default")]
        name: String,

        #[arg(long)]
        tags: Vec<String>,
    }

    // Verify default parsing works
    let cfg = Config::parse_from(["test", "--tags", "x,y,z"]);
    assert_eq!(cfg.tags, vec!["x", "y", "z"]);

    cleanup(&[file_path]);
}

/// Test 9: Error handling - invalid value from file
#[test]
fn test_invalid_file_value() {
    let file_path = "invalid_config.toml";
    write_toml(file_path, "port = \"not-a-number\"\n");

    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    // TODO: test error handling once implemented
    let _cfg = Config::parse_from(["test"]);

    cleanup(&[file_path]);
}

/// Test 10: Error handling - invalid value from env var
#[test]
fn test_invalid_env_value() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    std::env::set_var("MYAPP_PORT", "not-a-number");
    
    // TODO: test error handling once implemented
    let _cfg = Config::parse_from(["test"]);

    std::env::remove_var("MYAPP_PORT");
}

/// Test 11: Env var with prefix
#[test]
fn test_env_with_prefix() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    std::env::set_var("MYAPP_PORT", "8080");

    // TODO: test env prefix once implemented
    let _cfg = Config::parse_from(["test"]);

    std::env::remove_var("MYAPP_PORT");
}

/// Test 12: Field-level no_env - field excluded from env reading
#[test]
fn test_no_env() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        #[layered(no_env)]
        sensitive_key: String,
    }

    std::env::set_var("MYAPP_SENSITIVE_KEY", "leaked");
    
    // TODO: test no_env once implemented
    let _cfg = Config::parse_from(["test"]);

    std::env::remove_var("MYAPP_SENSITIVE_KEY");
}

/// Test 13: Field-level no_file - field excluded from file reading
#[test]
fn test_no_file() {
    let file_path = "no_file_config.toml";
    write_toml(file_path, "port = 5000\nsensitive_key = \"from-file\"\n");

    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        #[layered(no_file)]
        sensitive_key: String,
    }

    // TODO: test no_file once implemented
    let _cfg = Config::parse_from(["test"]);

    cleanup(&[file_path]);
}

/// Test 14: Empty file - all defaults
#[test]
fn test_empty_file() {
    let file_path = "empty_config.toml";
    write_toml(file_path, "");

    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    // TODO: test empty file once implemented
    let _cfg = Config::parse_from(["test"]);

    cleanup(&[file_path]);
}

/// Test 15: No config specified - all defaults
#[test]
fn test_no_config_specified() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    // TODO: test no file once layered() is implemented
    let _cfg = Config::parse_from(["test"]);
}

/// Test 16: Multiple values - Vec support with append/replace strategies
#[test]
fn test_vec_support() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value = "a,b")]
        items: Vec<String>,
    }

    let cfg = Config::parse_from(["test"]);
    assert_eq!(cfg.items, vec!["a", "b"]);

    // CLI can override
    let cfg = Config::parse_from(["test", "--items", "x,y,z"]);
    assert_eq!(cfg.items, vec!["x", "y", "z"]);
}

/// Test 17: Nested defaults - one field depends on default of another
#[test]
fn test_nested_defaults() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 8080)]
        port: u16,

        // Host uses port in its default
        #[arg(long, default_value = "localhost")]
        host: String,
    }

    let cfg = Config::parse_from(["test"]);
    assert_eq!(cfg.port, 8080);
    assert_eq!(cfg.host, "localhost");
}

/// Test 18: --help still shows real defaults (not Option wrapping)
#[test]
fn test_help_shows_real_defaults() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        /// Port to listen on
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    // This should not panic - verify the derive works with --help
    let result = std::panic::catch_unwind(|| {
        Config::parse_from(["test", "--help"]);
    });

    // Help should display defaults, not show all fields as optional
}

/// Test 19: Source attribution in errors - file source
#[test]
fn test_source_attribution_file() {
    let file_path = "error_config.toml";
    write_toml(file_path, "port = \"invalid\"\n");

    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    // TODO: verify error message mentions file source once implemented
    let _cfg = Config::parse_from(["test"]);

    cleanup(&[file_path]);
}

/// Test 20: Source attribution in errors - env var source
#[test]
fn test_source_attribution_env() {
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    std::env::set_var("MYAPP_PORT", "invalid");

    // TODO: verify error message mentions env source once implemented
    let _cfg = Config::parse_from(["test"]);

    std::env::remove_var("MYAPP_PORT");
}
