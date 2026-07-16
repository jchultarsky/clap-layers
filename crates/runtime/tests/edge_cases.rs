//! Edge cases that a naive implementation gets wrong.
//!
//! Each test here pins behaviour that was either found broken during
//! development, or is a trap that configuration libraries classically fall
//! into. Grouped by the kind of thing being stressed.

use clap::Parser;
use clap_layers::{Env, Layered, LayeredError};
use serde::Deserialize;

mod support;
use support::{TempToml, no_env};

/// Declare a single-`port` config bound to its own config file.
///
/// Every test needs a distinct file: the path is fixed at compile time, and
/// `TempToml` refuses to share one, so reusing a struct across tests would make
/// them clobber each other.
macro_rules! port_config {
    ($name:ident, $file:literal) => {
        #[derive(Parser, Layered, Debug)]
        #[layered(file = $file)]
        struct $name {
            #[arg(long, default_value_t = 3000)]
            port: u16,
        }
    };
}

// =====================================================================
// "Falsy but set": a value that equals its type's zero must still count
// as *set*. Treating `false`, `0`, `""` or `[]` as absent is the classic
// config-library bug, because the natural implementation is a truthiness
// check rather than a presence check.
// =====================================================================

macro_rules! falsy_config {
    ($name:ident, $file:literal) => {
        #[derive(Parser, Layered, Debug)]
        #[layered(file = $file, env_prefix = "EC_FALSY")]
        struct $name {
            /// Default is deliberately `true`, so `false` from a layer must win.
            #[arg(long, default_value_t = true)]
            verbose: bool,
            /// Default is non-zero, so `0` from a layer must win.
            #[arg(long, default_value_t = 3000)]
            port: u16,
            /// Default is non-empty, so `""` from a layer must win.
            #[arg(long, default_value_t = String::from("default-host"))]
            host: String,
            /// Default is non-empty, so `[]` from a layer must win.
            #[arg(long, value_delimiter = ',', default_value = "a,b")]
            tags: Vec<String>,
        }
    };
}

falsy_config!(FalsyFile, ".test-tmp/edge_falsy_file.toml");
falsy_config!(FalsyEnv, ".test-tmp/edge_falsy_env.toml");

#[test]
fn falsy_values_from_the_file_are_set_not_absent() {
    let _f = TempToml::new(
        "edge_falsy_file.toml",
        "verbose = false\nport = 0\nhost = \"\"\ntags = []\n",
    );

    let cfg = FalsyFile::layered_from(["t"], &no_env()).unwrap();

    assert!(
        !cfg.verbose,
        "`verbose = false` must override a `true` default"
    );
    assert_eq!(cfg.port, 0, "`port = 0` must override a non-zero default");
    assert_eq!(
        cfg.host, "",
        "`host = \"\"` must override a non-empty default"
    );
    assert!(
        cfg.tags.is_empty(),
        "`tags = []` must override a non-empty default"
    );
}

#[test]
fn falsy_values_from_the_environment_are_set_not_absent() {
    // The file sets the opposite of each default, so if the environment were
    // ignored these would take the file's values instead.
    let _f = TempToml::new(
        "edge_falsy_env.toml",
        "verbose = true\nport = 1\nhost = \"file\"\ntags = [\"f\"]\n",
    );

    let env = Env::from_iter([
        ("EC_FALSY_VERBOSE", "false"),
        ("EC_FALSY_PORT", "0"),
        ("EC_FALSY_HOST", ""),
        ("EC_FALSY_TAGS", "[]"),
    ]);
    let cfg = FalsyEnv::layered_from(["t"], &env).unwrap();

    assert!(!cfg.verbose);
    assert_eq!(cfg.port, 0);
    assert_eq!(cfg.host, "");
    assert!(cfg.tags.is_empty());
}

// =====================================================================
// Identifiers and hygiene: the generated code must not collide with, or
// mis-name, anything the user wrote.
// =====================================================================

/// Regression: a raw-identifier field used to panic.
///
/// `r#type` is the argument id `type` to clap. Passing the raw spelling to
/// `ArgMatches::value_source` queries an id clap never registered, which panics
/// in debug builds rather than returning `None`.
#[derive(Parser, Layered, Debug)]
#[layered(file = ".test-tmp/edge_raw.toml", env_prefix = "EC_RAW")]
struct RawIdents {
    #[arg(long, default_value_t = 1)]
    r#type: u16,

    #[arg(long, default_value_t = 2)]
    r#match: u16,
}

#[test]
fn raw_identifier_fields_resolve_on_every_layer() {
    let _f = TempToml::new("edge_raw.toml", "type = 50\nmatch = 60\n");

    // Default.
    assert_eq!(
        RawIdents::layered_from(["t"], &no_env()).unwrap().r#match,
        60
    );

    // File, keyed by the unraw name.
    assert_eq!(
        RawIdents::layered_from(["t"], &no_env()).unwrap().r#type,
        50
    );

    // Environment, named by the unraw field name.
    let env = Env::from_iter([("EC_RAW_TYPE", "70")]);
    assert_eq!(RawIdents::layered_from(["t"], &env).unwrap().r#type, 70);

    // An explicit flag still wins, and querying its ValueSource must not panic.
    assert_eq!(
        RawIdents::layered_from(["t", "--type", "80"], &env)
            .unwrap()
            .r#type,
        80
    );
}

/// Fields named after the locals the derive generates must still work.
#[derive(Parser, Layered, Debug)]
#[layered(env_prefix = "EC_HYG")]
struct Hygiene {
    #[arg(long, default_value_t = 1)]
    env: u16,
    #[arg(long, default_value_t = 2)]
    args: u16,
    #[arg(long, default_value_t = 3)]
    __cli: u16,
    #[arg(long, default_value_t = 4)]
    __matches: u16,
    #[arg(long, default_value_t = 5)]
    __file: u16,
}

#[test]
fn field_names_colliding_with_generated_locals() {
    let env = Env::from_iter([("EC_HYG_ENV", "9"), ("EC_HYG___FILE", "8")]);
    let cfg = Hygiene::layered_from(["t", "--args", "7"], &env).unwrap();

    assert_eq!(cfg.env, 9);
    assert_eq!(cfg.args, 7);
    assert_eq!(cfg.__cli, 3);
    assert_eq!(cfg.__matches, 4);
    assert_eq!(cfg.__file, 8);
}

// =====================================================================
// Source attribution: line numbers must survive anything the file does.
// =====================================================================

port_config!(Crlf, ".test-tmp/edge_crlf.toml");
port_config!(Multibyte, ".test-tmp/edge_multibyte.toml");
port_config!(Multiline, ".test-tmp/edge_multiline.toml");
port_config!(WrongType, ".test-tmp/edge_wrongtype.toml");

fn line_of_error(err: &LayeredError) -> usize {
    match err {
        LayeredError::Invalid {
            layer: clap_layers::SourceLayer::ConfigFile { line, .. },
            ..
        } => *line,
        other => panic!("expected a ConfigFile Invalid error, got {other:?}"),
    }
}

#[test]
fn line_numbers_survive_crlf_endings() {
    let _f = TempToml::new(
        "edge_crlf.toml",
        "# c\r\nhost = \"x\"\r\nport = \"bad\"\r\n",
    );
    let err = Crlf::layered_from(["t"], &no_env()).unwrap_err();
    assert_eq!(line_of_error(&err), 3);
}

#[test]
fn line_numbers_survive_multibyte_characters() {
    // Byte offsets and character counts diverge here; the line must not.
    let _f = TempToml::new(
        "edge_multibyte.toml",
        "# ★ ünicode ★\nname = \"日本語のテキスト\"\nport = \"bad\"\n",
    );
    let err = Multibyte::layered_from(["t"], &no_env()).unwrap_err();
    assert_eq!(line_of_error(&err), 3);
}

#[test]
fn line_numbers_survive_multiline_strings() {
    // A line-counting parser that walked key/value pairs would report line 2.
    // The value's real span puts it on line 5.
    let _f = TempToml::new(
        "edge_multiline.toml",
        "banner = \"\"\"\nline A\nline B\n\"\"\"\nport = \"bad\"\n",
    );
    let err = Multiline::layered_from(["t"], &no_env()).unwrap_err();
    assert_eq!(line_of_error(&err), 5);
}

#[test]
fn a_wrong_type_renders_the_value_as_written() {
    // A table or array where a scalar is expected must render readably rather
    // than panicking or dumping a debug representation.
    let f = TempToml::new("edge_wrongtype.toml", "");
    for (content, expected) in [
        ("port = [1, 2, 3]\n", "invalid value '[1, 2, 3]' for 'port'"),
        (
            "[port]\nnested = 1\n",
            "invalid value '{ nested = 1 }' for 'port'",
        ),
        ("port = 99999\n", "invalid value '99999' for 'port'"),
        ("port = -1\n", "invalid value '-1' for 'port'"),
        ("port = 1.5\n", "invalid value '1.5' for 'port'"),
        ("port = true\n", "invalid value 'true' for 'port'"),
    ] {
        f.rewrite(content);
        let err = WrongType::layered_from(["t"], &no_env()).unwrap_err();
        assert!(
            err.to_string().starts_with(expected),
            "for {content:?}\n  expected prefix: {expected}\n  got: {err}"
        );
    }
}

// =====================================================================
// TOML shapes.
// =====================================================================

#[derive(Deserialize, Debug, Default, Clone, PartialEq, Eq)]
struct Server {
    host: String,
    port: u16,
}

macro_rules! nested_config {
    ($name:ident, $file:literal) => {
        #[derive(Parser, Layered, Debug)]
        #[layered(file = $file, env_prefix = "EC_NEST")]
        struct $name {
            #[layered(no_cli)]
            #[arg(skip)]
            server: Server,
        }
    };
}

nested_config!(NestedFile, ".test-tmp/edge_nested_file.toml");
nested_config!(NestedEnv, ".test-tmp/edge_nested_env.toml");

#[test]
fn a_section_table_deserializes_into_a_nested_struct() {
    let _f = TempToml::new(
        "edge_nested_file.toml",
        "[server]\nhost = \"h\"\nport = 99\n",
    );

    let cfg = NestedFile::layered_from(["t"], &no_env()).unwrap();
    assert_eq!(cfg.server.host, "h");
    assert_eq!(cfg.server.port, 99);
}

#[test]
fn a_nested_struct_can_come_from_the_environment_as_an_inline_table() {
    let _f = TempToml::new(
        "edge_nested_env.toml",
        "[server]\nhost = \"file\"\nport = 1\n",
    );

    // Environment values are read as TOML, so an inline table works and beats
    // the file, as any other env value would.
    let env = Env::from_iter([("EC_NEST_SERVER", "{ host = \"env\", port = 2 }")]);
    let cfg = NestedEnv::layered_from(["t"], &env).unwrap();
    assert_eq!(cfg.server.host, "env");
    assert_eq!(cfg.server.port, 2);
}

port_config!(Emptyish, ".test-tmp/edge_emptyish.toml");
port_config!(Sections, ".test-tmp/edge_sections.toml");
port_config!(DupKey, ".test-tmp/edge_dupkey.toml");

#[test]
fn files_that_set_nothing_relevant_fall_through_to_defaults() {
    let f = TempToml::new("edge_emptyish.toml", "");
    for content in [
        "",                         // empty
        "\n\n   \n",                // whitespace only
        "# just a comment\n",       // comments only
        "unrelated = 1\n",          // keys we have no field for are ignored
        "[section]\nport = 5000\n", // our key, but nested under a table
    ] {
        f.rewrite(content);
        let cfg = Emptyish::layered_from(["t"], &no_env()).unwrap();
        assert_eq!(cfg.port, 3000, "for content {content:?}");
    }
}

/// A key under `[section]` must **not** be confused with a top-level key.
///
/// The previous line-splitting parser ignored table headers entirely, so
/// `[a].port` and `[b].port` silently collided with each other and with a
/// top-level `port`.
#[test]
fn section_headers_are_not_flattened_into_top_level_keys() {
    let _f = TempToml::new(
        "edge_sections.toml",
        "[server]\nport = 5000\n\n[client]\nport = 6000\n",
    );

    let cfg = Sections::layered_from(["t"], &no_env()).unwrap();
    assert_eq!(
        cfg.port, 3000,
        "a `port` under [server] is not the top-level `port`"
    );
}

#[test]
fn a_duplicate_key_is_a_parse_error() {
    let _f = TempToml::new("edge_dupkey.toml", "port = 1\nport = 2\n");
    let err = DupKey::layered_from(["t"], &no_env()).unwrap_err();
    assert!(matches!(err, LayeredError::Parse { .. }), "got: {err:?}");
}

// =====================================================================
// Environment value decoding.
// =====================================================================

#[derive(Parser, Layered, Debug)]
#[layered(env_prefix = "EC_ENV")]
struct EnvShapes {
    #[arg(long, default_value_t = String::from("d"))]
    host: String,
    #[arg(long, value_delimiter = ',', default_value = "x")]
    tags: Vec<String>,
    #[arg(long, default_value_t = 1)]
    port: u16,
}

#[test]
fn environment_strings_do_not_need_quoting() {
    for (raw, expected) in [
        ("localhost", "localhost"),
        ("hello world", "hello world"), // spaces
        ("127.0.0.1", "127.0.0.1"),     // not a TOML number
        ("true", "true"),               // a TOML bool, but the field is a String
        ("8080", "8080"),               // a TOML integer, likewise
        ("", ""),                       // set but empty
        ("  padded  ", "  padded  "),   // whitespace is preserved
        ("[not-an-array", "[not-an-array"),
    ] {
        let env = Env::from_iter([("EC_ENV_HOST", raw)]);
        let cfg = EnvShapes::layered_from(["t"], &env).unwrap();
        assert_eq!(cfg.host, expected, "for raw env value {raw:?}");
    }
}

#[test]
fn a_quoted_environment_string_is_read_as_toml() {
    // Documented consequence of reading env values as TOML: quotes are syntax.
    let env = Env::from_iter([("EC_ENV_HOST", "\"quoted\"")]);
    assert_eq!(
        EnvShapes::layered_from(["t"], &env).unwrap().host,
        "quoted",
        "a quoted env value is a TOML string, so the quotes are not literal"
    );
}

#[test]
fn a_sequence_from_the_environment_must_be_a_toml_array() {
    let env = Env::from_iter([("EC_ENV_TAGS", r#"["a", "b"]"#)]);
    assert_eq!(
        EnvShapes::layered_from(["t"], &env).unwrap().tags,
        ["a", "b"]
    );

    // A comma-separated list is *not* an array. clap's `value_delimiter` applies
    // to the command line only, so this is a clear error rather than a silently
    // single-element vector.
    let env = Env::from_iter([("EC_ENV_TAGS", "a,b")]);
    let err = EnvShapes::layered_from(["t"], &env).unwrap_err();
    assert!(
        err.to_string().contains("expected a sequence"),
        "got: {err}"
    );
}

#[test]
fn an_undecodable_environment_value_names_the_variable_not_the_field_type() {
    for raw in ["banana", "", "1.5", "-1", "99999", "[]"] {
        let env = Env::from_iter([("EC_ENV_PORT", raw)]);
        let err = EnvShapes::layered_from(["t"], &env).unwrap_err();
        assert!(
            err.to_string().contains("environment variable EC_ENV_PORT"),
            "for {raw:?}, got: {err}"
        );
    }
}

// =====================================================================
// clap features that must keep working underneath the layers.
// =====================================================================

#[derive(Parser, Layered, Debug)]
#[layered(env_prefix = "EC_COUNT")]
struct Counted {
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[test]
fn a_count_action_layers_like_any_other_field() {
    assert_eq!(Counted::layered_from(["t"], &no_env()).unwrap().verbose, 0);
    assert_eq!(
        Counted::layered_from(["t", "-vvv"], &no_env())
            .unwrap()
            .verbose,
        3
    );

    // Not passed: clap reports the count as a default, so lower layers apply.
    let env = Env::from_iter([("EC_COUNT_VERBOSE", "7")]);
    assert_eq!(Counted::layered_from(["t"], &env).unwrap().verbose, 7);

    // Passed: explicit wins.
    assert_eq!(
        Counted::layered_from(["t", "-vvv"], &env).unwrap().verbose,
        3
    );
}

#[derive(clap::Subcommand, Debug, PartialEq, Eq)]
enum Command {
    Run,
    Stop,
}

#[derive(Parser, Layered, Debug)]
#[layered(file = ".test-tmp/edge_sub.toml", env_prefix = "EC_SUB")]
struct WithSubcommand {
    #[arg(long, default_value_t = 3000)]
    port: u16,

    #[command(subcommand)]
    command: Command,
}

/// Subcommands are out of scope for layering, but a struct that has one must
/// still work: the field has no argument id, so resolving it as a scalar would
/// panic.
#[test]
fn a_subcommand_field_is_passed_through_while_siblings_still_layer() {
    let _f = TempToml::new("edge_sub.toml", "port = 5000\n");

    let cfg = WithSubcommand::layered_from(["t", "run"], &no_env()).unwrap();
    assert_eq!(cfg.command, Command::Run);
    assert_eq!(cfg.port, 5000, "the sibling field still layers");

    let env = Env::from_iter([("EC_SUB_PORT", "8080")]);
    let cfg = WithSubcommand::layered_from(["t", "stop"], &env).unwrap();
    assert_eq!(cfg.command, Command::Stop);
    assert_eq!(cfg.port, 8080);
}

#[derive(Parser, Layered, Debug)]
#[layered(file = ".test-tmp/edge_required.toml", env_prefix = "EC_REQ")]
struct Required {
    #[arg(long)]
    port: u16,
}

/// A documented limitation, pinned so it cannot change silently.
///
/// clap enforces required-ness while parsing, before any lower layer is
/// consulted, so a field with no default and no `Option` cannot be satisfied
/// from a file or the environment. Give such a field a default, or make it
/// `Option<T>`.
#[test]
fn a_required_field_cannot_be_satisfied_by_a_lower_layer() {
    // The documented workaround: `Option<T>` is not required, so the file layer
    // does apply. It shares `Required`'s file deliberately — both only read it.
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/edge_required.toml")]
    struct Optional {
        #[arg(long)]
        port: Option<u16>,
    }

    let _f = TempToml::new("edge_required.toml", "port = 5000\n");

    let err = Required::layered_from(["t"], &no_env()).unwrap_err();
    match err {
        LayeredError::Cli(e) => {
            assert_eq!(e.kind(), clap::error::ErrorKind::MissingRequiredArgument);
        }
        other => panic!("expected a Cli error, got {other:?}"),
    }

    assert_eq!(
        Optional::layered_from(["t"], &no_env()).unwrap().port,
        Some(5000)
    );
}

#[test]
fn multiple_layered_attributes_on_one_field_combine() {
    #[derive(Parser, Layered, Debug)]
    #[layered(file = ".test-tmp/edge_multiattr.toml", env_prefix = "EC_MULTI")]
    struct Config {
        // Split across two attributes rather than one list.
        #[layered(no_env)]
        #[layered(no_file)]
        #[arg(long, default_value_t = String::from("default"))]
        secret: String,
    }

    let _f = TempToml::new("edge_multiattr.toml", "secret = \"from-file\"\n");

    let env = Env::from_iter([("EC_MULTI_SECRET", "from-env")]);
    let cfg = Config::layered_from(["t"], &env).unwrap();
    assert_eq!(cfg.secret, "default", "both markers must apply");
}
