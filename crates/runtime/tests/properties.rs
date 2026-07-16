//! Property-based tests for the merge engine.
//!
//! The precedence matrix enumerates layer combinations by hand, which proves the
//! cases someone thought of. These generate layer stacks instead and assert the
//! rule itself: **the highest-priority layer that is set wins, and every lower
//! layer is invisible**.
//!
//! Each test drives the full `layered_from` path rather than the internal merge
//! helper, so a bug anywhere between the attribute and the resolved value is in
//! scope.
//!
//! `proptest` runs its cases sequentially within a test, but test functions run
//! in parallel, so each test here owns a distinct config file name.

use clap::Parser;
use clap_layers::{Env, Layered};
use proptest::prelude::*;

mod support;
use support::TempToml;

/// The built-in default every config below falls back to.
const DEFAULT_PORT: u16 = 3000;

/// Which layers set a value, and to what.
///
/// `None` means "this layer does not set the field at all", which is a
/// different thing from setting it to zero — see the `falsy` tests in
/// `edge_cases.rs`.
#[derive(Debug, Clone, Copy)]
struct Stack {
    flag: Option<u16>,
    env: Option<u16>,
    file: Option<u16>,
}

impl Stack {
    /// The rule, stated once: first layer that is set, else the default.
    fn expected(self) -> u16 {
        self.flag.or(self.env).or(self.file).unwrap_or(DEFAULT_PORT)
    }
}

fn port_value() -> impl Strategy<Value = u16> {
    prop_oneof![
        // Weighted so that "the layer's value happens to equal the default" —
        // the exact case this crate exists to get right — comes up constantly
        // rather than once every 65536 draws.
        1 => Just(DEFAULT_PORT),
        3 => any::<u16>(),
    ]
}

fn maybe_port() -> impl Strategy<Value = Option<u16>> {
    prop_oneof![Just(None), port_value().prop_map(Some)]
}

fn stack() -> impl Strategy<Value = Stack> {
    (maybe_port(), maybe_port(), maybe_port()).prop_map(|(flag, env, file)| Stack {
        flag,
        env,
        file,
    })
}

/// Build the argument list a `Stack` implies.
fn args_for(flag: Option<u16>) -> Vec<String> {
    let mut args = vec!["app".to_string()];
    if let Some(value) = flag {
        args.push("--port".to_string());
        args.push(value.to_string());
    }
    args
}

fn env_for(prefix: &str, env: Option<u16>) -> Env {
    env.map_or_else(Env::empty, |value| {
        Env::from_iter([(format!("{prefix}_PORT"), value.to_string())])
    })
}

macro_rules! port_config {
    ($name:ident, $file:literal, $prefix:literal $(, $marker:meta)?) => {
        #[derive(Parser, Layered, Debug)]
        #[layered(file = $file, env_prefix = $prefix)]
        struct $name {
            $(#[$marker])?
            #[arg(long, default_value_t = DEFAULT_PORT)]
            port: u16,
        }
    };
}

port_config!(AllLayers, ".test-tmp/prop_all.toml", "PROP_ALL");
// A distinct struct, and so a distinct file, because test functions run in
// parallel and `TempToml` refuses to share one.
port_config!(Disturb, ".test-tmp/prop_disturb.toml", "PROP_DIST");
port_config!(
    NoEnv,
    ".test-tmp/prop_no_env.toml",
    "PROP_NOENV",
    layered(no_env)
);
port_config!(
    NoFile,
    ".test-tmp/prop_no_file.toml",
    "PROP_NOFILE",
    layered(no_file)
);

proptest! {
    /// The core rule, over every combination of layers being set or unset.
    #[test]
    fn the_highest_priority_layer_that_is_set_wins(stack in stack()) {
        let _file = stack
            .file
            .map(|v| TempToml::new("prop_all.toml", &format!("port = {v}\n")));

        let cfg = AllLayers::layered_from(
            args_for(stack.flag),
            &env_for("PROP_ALL", stack.env),
        )?;

        prop_assert_eq!(cfg.port, stack.expected(), "for {:?}", stack);
    }

    /// `no_env` must make the environment layer invisible — not merely
    /// lower-priority. The expectation drops `env` from the rule entirely.
    #[test]
    fn no_env_removes_the_environment_layer(stack in stack()) {
        let _file = stack
            .file
            .map(|v| TempToml::new("prop_no_env.toml", &format!("port = {v}\n")));

        let cfg = NoEnv::layered_from(
            args_for(stack.flag),
            &env_for("PROP_NOENV", stack.env),
        )?;

        let expected = stack.flag.or(stack.file).unwrap_or(DEFAULT_PORT);
        prop_assert_eq!(cfg.port, expected, "for {:?}", stack);
    }

    /// The same for `no_file`.
    #[test]
    fn no_file_removes_the_file_layer(stack in stack()) {
        let _file = stack
            .file
            .map(|v| TempToml::new("prop_no_file.toml", &format!("port = {v}\n")));

        let cfg = NoFile::layered_from(
            args_for(stack.flag),
            &env_for("PROP_NOFILE", stack.env),
        )?;

        let expected = stack.flag.or(stack.env).unwrap_or(DEFAULT_PORT);
        prop_assert_eq!(cfg.port, expected, "for {:?}", stack);
    }

    /// Adding a *lower* layer can never change an already-decided value.
    ///
    /// Stated separately from the rule above because it is the property users
    /// actually rely on: adding a config file must not disturb a working
    /// command line.
    #[test]
    fn a_lower_layer_cannot_disturb_a_higher_one(
        flag in maybe_port(),
        env in maybe_port(),
        file_a in port_value(),
        file_b in port_value(),
    ) {
        // Skip the case where the file is the highest layer set; there, the
        // file legitimately decides the value.
        prop_assume!(flag.is_some() || env.is_some());

        let with_a = {
            let _f = TempToml::new("prop_disturb.toml", &format!("port = {file_a}\n"));
            Disturb::layered_from(args_for(flag), &env_for("PROP_DIST", env))?.port
        };
        let with_b = {
            let _f = TempToml::new("prop_disturb.toml", &format!("port = {file_b}\n"));
            Disturb::layered_from(args_for(flag), &env_for("PROP_DIST", env))?.port
        };

        prop_assert_eq!(with_a, with_b, "the file layer changed a value it should not own");
    }
}

// ---------------------------------------------------------------- round-trips

#[derive(Parser, Layered, Debug)]
#[layered(file = ".test-tmp/prop_file_string.toml")]
struct FileString {
    #[arg(long, default_value_t = String::new())]
    host: String,
}

#[derive(Parser, Layered, Debug)]
#[layered(env_prefix = "PROP_STR")]
struct EnvString {
    #[arg(long, default_value_t = String::new())]
    host: String,
}

proptest! {
    /// Any string written to the file by `toml` must come back byte-identical.
    ///
    /// Serialising with `toml` and parsing with our loader exercises quoting,
    /// escaping and multi-line handling for values a hand-written test would
    /// never think to try.
    #[test]
    fn a_string_survives_the_file_layer_verbatim(value in any::<String>()) {
        let mut table = toml::Table::new();
        table.insert("host".to_string(), toml::Value::String(value.clone()));
        let content = toml::to_string(&table).expect("toml should serialise a string");

        let _file = TempToml::new("prop_file_string.toml", &content);
        let cfg = FileString::layered_from(["app"], &Env::empty())?;

        prop_assert_eq!(cfg.host, value);
    }

    /// A bare environment value arrives verbatim.
    ///
    /// Environment values are read as TOML so that `Vec<T>` and friends work at
    /// all, with a fallback to a plain string. That fallback is what users lean
    /// on for `MYAPP_HOST=localhost`, and it must not mangle anything.
    ///
    /// Values whose *trimmed* form opens with a quote are excluded: those are
    /// valid TOML string expressions, so the quotes are syntax and are stripped.
    /// That is a documented consequence, covered by
    /// `edge_cases::a_quoted_environment_string_is_read_as_toml`.
    #[test]
    fn a_bare_environment_string_arrives_verbatim(
        value in any::<String>()
            .prop_filter(
                "a quoted value is a TOML string expression, not a bare string",
                |v| !v.trim_start().starts_with(['"', '\'']),
            ),
    ) {
        let env = Env::from_iter([("PROP_STR_HOST", value.clone())]);
        let cfg = EnvString::layered_from(["app"], &env)?;

        prop_assert_eq!(cfg.host, value);
    }
}
