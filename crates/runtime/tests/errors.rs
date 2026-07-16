//! Source-attributed errors.
//!
//! A bad value must name the layer that supplied it. The previous
//! implementation silently discarded parse failures and fell back to the
//! default, so these tests assert on the *message*, not merely that an error
//! occurred.

use clap::Parser;
use clap_layers::{Env, Layered, LayeredError, SourceLayer};

mod support;
use support::{TempToml, no_env};

#[test]
fn bad_file_value_names_the_file_and_line() {
    // `port` is deliberately on line 4, after a comment and a blank line, so a
    // hard-coded or off-by-one line number would fail.
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/errors_bad_value.toml")]
    struct Config {
        #[arg(long, default_value_t = String::from("h"))]
        host: String,
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    let _f = TempToml::new(
        "errors_bad_value.toml",
        "# a comment\n\nhost = \"ok\"\nport = \"not-a-number\"\n",
    );

    let err = Config::layered_from(["t"], &no_env()).unwrap_err();

    // The exact shape promised by the project's correctness bar.
    let msg = err.to_string();
    assert!(
        msg.starts_with("invalid value 'not-a-number' for 'port' — from"),
        "got: {msg}"
    );
    assert!(msg.contains("errors_bad_value.toml, line 4"), "got: {msg}");

    match err {
        LayeredError::Invalid {
            field,
            value,
            layer,
            ..
        } => {
            assert_eq!(field, "port");
            assert_eq!(value, "not-a-number");
            match layer {
                SourceLayer::ConfigFile { line, path } => {
                    assert_eq!(line, 4);
                    assert!(path.ends_with("errors_bad_value.toml"));
                }
                other => panic!("expected a ConfigFile layer, got {other:?}"),
            }
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn bad_env_value_names_the_variable() {
    #[derive(Parser, Layered, Debug)]
    #[layered(env_prefix = "ERR_ENV")]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    let env = Env::from_iter([("ERR_ENV_PORT", "banana")]);
    let err = Config::layered_from(["t"], &env).unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.starts_with(
            "invalid value 'banana' for 'port' — from environment variable ERR_ENV_PORT"
        ),
        "got: {msg}"
    );
    assert!(matches!(
        err,
        LayeredError::Invalid {
            layer: SourceLayer::EnvVar(_),
            ..
        }
    ));
}

/// A bad value must be a hard error, not a silent fall-through to the default.
#[test]
fn a_bad_value_never_silently_falls_back() {
    #[derive(Parser, Layered, Debug)]
    #[layered(env_prefix = "ERR_FALLBACK")]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    let env = Env::from_iter([("ERR_FALLBACK_PORT", "banana")]);
    assert!(
        Config::layered_from(["t"], &env).is_err(),
        "a malformed env var must not quietly resolve to the default"
    );
}

#[test]
fn malformed_toml_reports_line_and_column() {
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/errors_malformed.toml")]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    let _f = TempToml::new("errors_malformed.toml", "host = \"ok\"\nport = = 3\n");

    let err = Config::layered_from(["t"], &no_env()).unwrap_err();
    match err {
        LayeredError::Parse { line, path, .. } => {
            assert_eq!(line, 2);
            assert!(path.ends_with("errors_malformed.toml"));
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn a_missing_config_file_is_not_an_error() {
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/definitely-does-not-exist.toml")]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    // Config files are optional; the layer is simply skipped.
    assert_eq!(Config::layered_from(["t"], &no_env()).unwrap().port, 3000);
}

#[test]
fn an_unreadable_config_file_is_an_error() {
    // A file that exists but cannot be read is a misconfiguration, and must not
    // be confused with an absent one.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        #[derive(Parser, Layered, Debug)]
        #[layered(file = ".test-tmp/errors_unreadable.toml")]
        struct Config {
            #[arg(long, default_value_t = 3000)]
            port: u16,
        }

        let _f = TempToml::new("errors_unreadable.toml", "port = 5000\n");
        let path = std::path::Path::new(".test-tmp/errors_unreadable.toml");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();

        // Permission bits do not stop root, so only assert if the chmod
        // actually made the file unreadable for this user.
        let genuinely_unreadable = std::fs::read_to_string(path).is_err();
        let result = Config::layered_from(["t"], &no_env());

        // Restore permissions so the TempToml guard can clean up.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();

        if genuinely_unreadable {
            match result {
                Err(LayeredError::Io { path, .. }) => {
                    assert!(path.ends_with("errors_unreadable.toml"));
                }
                other => panic!("expected Io, got {other:?}"),
            }
        }
    }
}

#[test]
fn help_is_reported_as_a_cli_error_rather_than_exiting() {
    // `layered_from` must never exit the process: that is what makes it
    // testable. `layered()` is the one that defers to clap's exit behaviour.
    #[derive(Parser, Layered, Debug)]
    struct Config {
        /// Port to listen on
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    let err = Config::layered_from(["t", "--help"], &no_env()).unwrap_err();
    match err {
        LayeredError::Cli(e) => {
            assert_eq!(e.kind(), clap::error::ErrorKind::DisplayHelp);
            // Requirement: --help still shows real defaults, not `None`.
            assert!(e.to_string().contains("[default: 3000]"), "got: {e}");
        }
        other => panic!("expected Cli, got {other:?}"),
    }
}

/// A non-string TOML value must be rendered as written when it is the wrong
/// type, so the message quotes `true` rather than a debug representation.
#[test]
fn bad_file_value_of_a_non_string_type_is_rendered_as_written() {
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/errors_wrong_type.toml")]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    let _f = TempToml::new("errors_wrong_type.toml", "port = true\n");

    let err = Config::layered_from(["t"], &no_env()).unwrap_err();
    assert!(
        err.to_string()
            .starts_with("invalid value 'true' for 'port' — from"),
        "got: {err}"
    );
}
