//! The precedence matrix: `(flag | env | file | none)` × field type.
//!
//! This is the crate's flagship test. Every case calls [`Layered::layered_from`]
//! with an explicit argument list and an explicit [`Env`], so nothing depends on
//! the ambient process state and the whole file is safe to run in parallel.
//!
//! Each field type is exercised at all four layers:
//!
//! | Case      | Setup                                    | Expectation             |
//! | --------- | ---------------------------------------- | ----------------------- |
//! | `none`    | nothing set                              | the built-in default    |
//! | `file`    | config file sets it                      | file beats default      |
//! | `env`     | env **and** file set it                  | env beats file          |
//! | `flag`    | flag, env **and** file all set it        | flag beats everything   |
//!
//! The `flag` row deliberately leaves the lower layers populated, so a passing
//! result means the flag *won*, not merely that nothing else was there.

use clap::{Parser, ValueEnum};
use clap_layers::{Env, Layered};
use serde::Deserialize;

mod support;
use support::{TempToml, no_env};

// ---------------------------------------------------------------- u16

#[derive(Parser, Layered, Debug)]
#[layered(file = ".test-tmp/matrix_u16.toml", env_prefix = "MX_U16")]
struct U16File {
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

#[derive(Parser, Layered, Debug)]
#[layered(env_prefix = "MX_U16")]
struct U16NoFile {
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

#[test]
fn matrix_u16() {
    let _f = TempToml::new("matrix_u16.toml", "port = 5000\n");

    // none -> default
    assert_eq!(
        U16NoFile::layered_from(["t"], &no_env()).unwrap().port,
        3000
    );

    // file -> beats default
    assert_eq!(U16File::layered_from(["t"], &no_env()).unwrap().port, 5000);

    // env -> beats file
    let env = Env::from_iter([("MX_U16_PORT", "8080")]);
    assert_eq!(U16File::layered_from(["t"], &env).unwrap().port, 8080);

    // flag -> beats env and file
    assert_eq!(
        U16File::layered_from(["t", "--port", "9000"], &env)
            .unwrap()
            .port,
        9000
    );
}

// ---------------------------------------------------------------- bool

#[derive(Parser, Layered, Debug)]
#[layered(file = ".test-tmp/matrix_bool.toml", env_prefix = "MX_BOOL")]
struct BoolFile {
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

#[derive(Parser, Layered, Debug)]
#[layered(env_prefix = "MX_BOOL")]
struct BoolNoFile {
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

#[test]
fn matrix_bool() {
    let _f = TempToml::new("matrix_bool.toml", "verbose = true\n");

    assert!(!BoolNoFile::layered_from(["t"], &no_env()).unwrap().verbose);
    assert!(BoolFile::layered_from(["t"], &no_env()).unwrap().verbose);

    // env can turn the file's `true` back off.
    let env = Env::from_iter([("MX_BOOL_VERBOSE", "false")]);
    assert!(!BoolFile::layered_from(["t"], &env).unwrap().verbose);

    // ...and the flag overrides both.
    assert!(
        BoolFile::layered_from(["t", "--verbose"], &env)
            .unwrap()
            .verbose
    );
}

// ---------------------------------------------------------------- String

#[derive(Parser, Layered, Debug)]
#[layered(file = ".test-tmp/matrix_string.toml", env_prefix = "MX_STR")]
struct StringFile {
    #[arg(long, default_value_t = String::from("default-host"))]
    host: String,
}

#[derive(Parser, Layered, Debug)]
#[layered(env_prefix = "MX_STR")]
struct StringNoFile {
    #[arg(long, default_value_t = String::from("default-host"))]
    host: String,
}

#[test]
fn matrix_string() {
    let _f = TempToml::new("matrix_string.toml", "host = \"file-host\"\n");

    assert_eq!(
        StringNoFile::layered_from(["t"], &no_env()).unwrap().host,
        "default-host"
    );
    assert_eq!(
        StringFile::layered_from(["t"], &no_env()).unwrap().host,
        "file-host"
    );

    // An unquoted environment value is still a plain string.
    let env = Env::from_iter([("MX_STR_HOST", "env-host")]);
    assert_eq!(
        StringFile::layered_from(["t"], &env).unwrap().host,
        "env-host"
    );
    assert_eq!(
        StringFile::layered_from(["t", "--host", "flag-host"], &env)
            .unwrap()
            .host,
        "flag-host"
    );
}

// ---------------------------------------------------------------- Vec<T>

#[derive(Parser, Layered, Debug)]
#[layered(file = ".test-tmp/matrix_vec.toml", env_prefix = "MX_VEC")]
struct VecFile {
    #[arg(long, value_delimiter = ',', default_value = "a,b")]
    tags: Vec<String>,
}

#[derive(Parser, Layered, Debug)]
#[layered(env_prefix = "MX_VEC")]
struct VecNoFile {
    #[arg(long, value_delimiter = ',', default_value = "a,b")]
    tags: Vec<String>,
}

#[test]
fn matrix_vec() {
    let _f = TempToml::new("matrix_vec.toml", "tags = [\"p\", \"q\"]\n");

    assert_eq!(
        VecNoFile::layered_from(["t"], &no_env()).unwrap().tags,
        ["a", "b"]
    );
    assert_eq!(
        VecFile::layered_from(["t"], &no_env()).unwrap().tags,
        ["p", "q"]
    );

    // Environment values are read as TOML, so arrays are expressible.
    let env = Env::from_iter([("MX_VEC_TAGS", r#"["x", "y"]"#)]);
    assert_eq!(VecFile::layered_from(["t"], &env).unwrap().tags, ["x", "y"]);

    assert_eq!(
        VecFile::layered_from(["t", "--tags", "1,2,3"], &env)
            .unwrap()
            .tags,
        ["1", "2", "3"]
    );
}

// ---------------------------------------------------------------- Option<T>

#[derive(Parser, Layered, Debug)]
#[layered(file = ".test-tmp/matrix_option.toml", env_prefix = "MX_OPT")]
struct OptionFile {
    #[arg(long)]
    timeout: Option<u16>,
}

#[derive(Parser, Layered, Debug)]
#[layered(env_prefix = "MX_OPT")]
struct OptionNoFile {
    #[arg(long)]
    timeout: Option<u16>,
}

#[test]
fn matrix_option() {
    let _f = TempToml::new("matrix_option.toml", "timeout = 5000\n");

    // An unset `Option` field has no clap value at all, rather than a default.
    assert_eq!(
        OptionNoFile::layered_from(["t"], &no_env())
            .unwrap()
            .timeout,
        None
    );
    assert_eq!(
        OptionFile::layered_from(["t"], &no_env()).unwrap().timeout,
        Some(5000)
    );

    let env = Env::from_iter([("MX_OPT_TIMEOUT", "8080")]);
    assert_eq!(
        OptionFile::layered_from(["t"], &env).unwrap().timeout,
        Some(8080)
    );
    assert_eq!(
        OptionFile::layered_from(["t", "--timeout", "9000"], &env)
            .unwrap()
            .timeout,
        Some(9000)
    );
}

// ---------------------------------------------------------------- enum

#[derive(ValueEnum, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Mode {
    Fast,
    Slow,
    ExtraSlow,
}

#[derive(Parser, Layered, Debug)]
#[layered(file = ".test-tmp/matrix_enum.toml", env_prefix = "MX_ENUM")]
struct EnumFile {
    #[arg(long, value_enum, default_value_t = Mode::Fast)]
    mode: Mode,
}

#[derive(Parser, Layered, Debug)]
#[layered(env_prefix = "MX_ENUM")]
struct EnumNoFile {
    #[arg(long, value_enum, default_value_t = Mode::Fast)]
    mode: Mode,
}

#[test]
fn matrix_enum() {
    let _f = TempToml::new("matrix_enum.toml", "mode = \"slow\"\n");

    assert_eq!(
        EnumNoFile::layered_from(["t"], &no_env()).unwrap().mode,
        Mode::Fast
    );
    assert_eq!(
        EnumFile::layered_from(["t"], &no_env()).unwrap().mode,
        Mode::Slow
    );

    let env = Env::from_iter([("MX_ENUM_MODE", "extra-slow")]);
    assert_eq!(
        EnumFile::layered_from(["t"], &env).unwrap().mode,
        Mode::ExtraSlow
    );
    assert_eq!(
        EnumFile::layered_from(["t", "--mode", "fast"], &env)
            .unwrap()
            .mode,
        Mode::Fast
    );
}

// ------------------------------------------------- the flagship correctness case

/// The reason this crate exists.
///
/// A flag the user typed must beat every lower layer **even when the value they
/// typed is exactly the built-in default**. Detecting this requires clap's
/// `ValueSource`; any implementation that compares against the default value
/// fails here.
#[test]
fn explicit_flag_equal_to_default_still_beats_env_and_file() {
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/matrix_equal.toml", env_prefix = "MX_EQ")]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    let _f = TempToml::new("matrix_equal.toml", "port = 5000\n");

    let env = Env::from_iter([("MX_EQ_PORT", "8080")]);

    // Sanity: without the flag, the environment wins.
    assert_eq!(Config::layered_from(["t"], &env).unwrap().port, 8080);

    // Typing `--port 3000` is an explicit choice, even though 3000 is also the
    // default. It must beat both the env var and the file.
    let cfg = Config::layered_from(["t", "--port", "3000"], &env).unwrap();
    assert_eq!(
        cfg.port, 3000,
        "explicitly typing the default value must still beat env and file"
    );
}

/// The mirror image: a *default* must lose to the file, so we are not simply
/// treating every clap value as explicit.
#[test]
fn untouched_default_loses_to_every_layer() {
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/matrix_untouched.toml")]
    struct Config {
        #[arg(long, default_value_t = 3000)]
        port: u16,
    }

    let _f = TempToml::new("matrix_untouched.toml", "port = 5000\n");

    assert_eq!(Config::layered_from(["t"], &no_env()).unwrap().port, 5000);
}

// ---------------------------------------------------------------- mixed sources

/// Each field independently resolves to a different layer in a single parse.
#[test]
fn fields_resolve_independently() {
    // The shared `from_` prefix names each field after the layer it should
    // resolve to, which is the entire point of this test.
    #[allow(clippy::struct_field_names)]
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/matrix_mixed.toml", env_prefix = "MX_MIX")]
    struct Config {
        #[arg(long, default_value_t = String::from("default"))]
        from_default: String,
        #[arg(long, default_value_t = String::from("default"))]
        from_file: String,
        #[arg(long, default_value_t = String::from("default"))]
        from_env: String,
        #[arg(long, default_value_t = String::from("default"))]
        from_flag: String,
    }

    let _f = TempToml::new(
        "matrix_mixed.toml",
        "from_file = \"file\"\nfrom_env = \"file\"\nfrom_flag = \"file\"\n",
    );

    let env = Env::from_iter([("MX_MIX_FROM_ENV", "env"), ("MX_MIX_FROM_FLAG", "env")]);
    let cfg = Config::layered_from(["t", "--from-flag", "flag"], &env).unwrap();

    assert_eq!(cfg.from_default, "default");
    assert_eq!(cfg.from_file, "file");
    assert_eq!(cfg.from_env, "env");
    assert_eq!(cfg.from_flag, "flag");
}
