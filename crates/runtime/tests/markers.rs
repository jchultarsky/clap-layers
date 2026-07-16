//! Per-field layer markers: `no_cli`, `no_env`, `no_file`.
//!
//! These are a security-relevant feature — they are how a password is kept out
//! of the environment — so each marker is tested by proving the excluded layer
//! is genuinely ignored while a *neighbouring* field still reads it. A marker
//! that silently did nothing would pass a weaker test.

use clap::Parser;
use clap_layers::{Env, Layered, LayeredError};

mod support;
use support::{TempToml, no_env};

#[test]
fn no_env_ignores_the_environment_layer() {
    #[derive(Parser, Layered, Debug)]
    #[layered(env_prefix = "MK_NOENV")]
    struct Config {
        #[arg(long, default_value_t = String::from("default-user"))]
        user: String,

        #[layered(no_env)]
        #[arg(long, default_value_t = String::from("default-secret"))]
        secret: String,
    }

    let env = Env::from_iter([("MK_NOENV_USER", "env-user"), ("MK_NOENV_SECRET", "leaked")]);
    let cfg = Config::layered_from(["t"], &env).unwrap();

    // The control field proves the env layer is working at all...
    assert_eq!(cfg.user, "env-user");
    // ...so this genuinely demonstrates `no_env` excluding it.
    assert_eq!(
        cfg.secret, "default-secret",
        "no_env field read the environment"
    );
}

#[test]
fn no_file_ignores_the_file_layer() {
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/markers_no_file.toml")]
    struct Config {
        #[arg(long, default_value_t = String::from("default-user"))]
        user: String,

        #[layered(no_file)]
        #[arg(long, default_value_t = String::from("default-secret"))]
        secret: String,
    }

    let _f = TempToml::new(
        "markers_no_file.toml",
        "user = \"file-user\"\nsecret = \"leaked\"\n",
    );

    let cfg = Config::layered_from(["t"], &no_env()).unwrap();

    assert_eq!(cfg.user, "file-user");
    assert_eq!(cfg.secret, "default-secret", "no_file field read the file");
}

#[test]
fn no_env_and_no_file_leave_only_the_command_line() {
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/markers_cli_only.toml", env_prefix = "MK_BOTH")]
    struct Config {
        #[layered(no_env, no_file)]
        #[arg(long)]
        password: String,
    }

    let _f = TempToml::new("markers_cli_only.toml", "password = \"from-file\"\n");

    let env = Env::from_iter([("MK_BOTH_PASSWORD", "from-env")]);
    let cfg = Config::layered_from(["t", "--password", "typed"], &env).unwrap();
    assert_eq!(cfg.password, "typed");

    // With every other layer excluded and no default, omitting the flag is an
    // error rather than a silent fallback to the file or environment value.
    let err = Config::layered_from(["t"], &env).unwrap_err();
    assert!(matches!(err, LayeredError::Cli(_)), "got: {err:?}");
}

#[test]
fn no_cli_field_is_not_a_flag_but_still_layers() {
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/markers_no_cli.toml", env_prefix = "MK_NOCLI")]
    struct Config {
        #[layered(no_cli)]
        #[arg(skip = String::from("built-in"))]
        instance_id: String,
    }

    let _f = TempToml::new("markers_no_cli.toml", "instance_id = \"from-file\"\n");

    // The file layer still applies...
    let cfg = Config::layered_from(["t"], &no_env()).unwrap();
    assert_eq!(cfg.instance_id, "from-file");

    // ...and so does the environment, which beats the file.
    let env = Env::from_iter([("MK_NOCLI_INSTANCE_ID", "from-env")]);
    let cfg = Config::layered_from(["t"], &env).unwrap();
    assert_eq!(cfg.instance_id, "from-env");

    // But it is not a flag: clap rejects it as unknown.
    let err = Config::layered_from(["t", "--instance-id", "x"], &no_env()).unwrap_err();
    assert!(matches!(err, LayeredError::Cli(_)), "got: {err:?}");
}

#[test]
fn arg_skip_without_no_cli_does_not_panic_on_value_source() {
    // `ArgMatches::value_source` panics in debug builds for an unknown id, so a
    // skipped field must never be queried. `#[arg(skip)]` alone has to be
    // enough; requiring `no_cli` to avoid a panic would be a trap.
    #[derive(Parser, Layered, Debug)]
    #[layered(env_prefix = "MK_SKIP")]
    struct Config {
        #[arg(skip = 7u16)]
        internal: u16,
    }

    let cfg = Config::layered_from(["t"], &no_env()).unwrap();
    assert_eq!(cfg.internal, 7);

    let env = Env::from_iter([("MK_SKIP_INTERNAL", "42")]);
    assert_eq!(Config::layered_from(["t"], &env).unwrap().internal, 42);
}

#[test]
fn env_layer_is_disabled_without_a_prefix() {
    // Without `env_prefix` a field named `path` would otherwise read the
    // ambient PATH. The layer is off rather than guessing.
    #[derive(Parser, Layered, Debug)]
    struct Config {
        #[arg(long, default_value_t = String::from("default"))]
        path: String,
    }

    let env = Env::from_iter([("PATH", "/should/be/ignored"), ("path", "/also/ignored")]);
    assert_eq!(Config::layered_from(["t"], &env).unwrap().path, "default");
}

#[test]
fn flattened_structs_are_passed_through_untouched() {
    // A `#[command(flatten)]` field has no argument id of its own; querying its
    // ValueSource would panic. It must be passed straight through.
    #[derive(clap::Args, Debug)]
    struct Nested {
        #[arg(long, default_value_t = 5)]
        retries: u8,
    }

    #[derive(Parser, Layered, Debug)]
    #[layered(env_prefix = "MK_FLAT")]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,

        #[command(flatten)]
        nested: Nested,
    }

    let cfg = Config::layered_from(["t", "--retries", "9"], &no_env()).unwrap();
    assert_eq!(cfg.nested.retries, 9);
    assert_eq!(cfg.port, 3000);
}

#[test]
fn custom_arg_id_is_honoured() {
    // `#[arg(id = "...")]` renames the id we must query for ValueSource, while
    // the flag and the field keep their own names. All three differ here, so
    // querying the wrong one would panic or misreport.
    #[derive(Parser, Layered, Debug)]
    #[layered(env_prefix = "MK_ID")]
    struct Config {
        #[arg(id = "renamed", long = "port", default_value_t = 1)]
        port: u16,
    }

    let env = Env::from_iter([("MK_ID_PORT", "8080")]);
    assert_eq!(Config::layered_from(["t"], &env).unwrap().port, 8080);
    assert_eq!(
        Config::layered_from(["t", "--port", "9000"], &env)
            .unwrap()
            .port,
        9000
    );
}
