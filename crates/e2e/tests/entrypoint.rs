//! End-to-end tests for `Layered::layered`, the entry point applications call.
//!
//! `layered()` reads the real process arguments and environment, and hands CLI
//! errors to `clap::Error::exit`, which terminates the process. Neither is
//! reachable in-process, so these tests run a real binary and inspect its
//! stdout, stderr and exit status.
//!
//! Everything beneath `layered()` is covered by the runtime crate's in-process
//! suites; this file exists so the one function every user actually calls is not
//! taken on trust.

use std::process::{Command, Output};

/// Cargo builds this binary before the test and hands us its absolute path, so
/// there is no path under `target/` to guess at.
const APP: &str = env!("CARGO_BIN_EXE_clap-layers-e2e");

/// Run the app with the given arguments and environment.
///
/// The environment is cleared first, so a variable set in the developer's shell
/// or on CI cannot leak in and change the result.
fn run(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(APP);
    cmd.args(args).env_clear();

    // `env_clear` also drops the path coverage instrumentation writes to, which
    // would silently discard this subprocess's coverage — and this subprocess is
    // the only thing that exercises `layered()`. Put it back.
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        cmd.env("LLVM_PROFILE_FILE", profile);
    }

    for (key, value) in env {
        cmd.env(key, value);
    }
    cmd.output().expect("could not run the e2e binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Assert the app succeeded and printed `key=value`.
fn assert_field(output: &Output, expected: &str) {
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstderr: {}",
        output.status,
        stderr(output)
    );
    assert!(
        stdout(output).lines().any(|line| line == expected),
        "expected a line {expected:?}, got:\n{}",
        stdout(output)
    );
}

#[test]
fn defaults_apply_when_nothing_is_set() {
    let out = run(&[], &[]);
    assert_field(&out, "host=127.0.0.1");
    assert_field(&out, "port=3000");
    assert_field(&out, "verbose=false");
}

#[test]
fn the_real_process_environment_is_read() {
    let out = run(
        &[],
        &[
            ("MYAPP_PORT", "8080"),
            ("MYAPP_HOST", "example.com"),
            ("MYAPP_VERBOSE", "true"),
        ],
    );
    assert_field(&out, "host=example.com");
    assert_field(&out, "port=8080");
    assert_field(&out, "verbose=true");
}

#[test]
fn an_unrelated_variable_is_ignored() {
    // Only `MYAPP_`-prefixed names are ours. A field named `host` must not pick
    // up an ambient `HOST`.
    let out = run(&[], &[("HOST", "leaked"), ("PORT", "9999")]);
    assert_field(&out, "host=127.0.0.1");
    assert_field(&out, "port=3000");
}

#[test]
fn a_real_flag_beats_the_real_environment() {
    let out = run(&["--port", "9000"], &[("MYAPP_PORT", "8080")]);
    assert_field(&out, "port=9000");
}

/// The flagship case, proven against a real process rather than injected state.
#[test]
fn a_flag_equal_to_the_default_still_beats_the_environment() {
    let out = run(&["--port", "3000"], &[("MYAPP_PORT", "8080")]);
    assert_field(&out, "port=3000");
}

/// `--help` prints to stdout and exits 0, exactly as `clap::Parser::parse`
/// does, rather than surfacing as a configuration error.
#[test]
fn help_goes_to_stdout_and_exits_zero() {
    let out = run(&["--help"], &[]);
    assert!(
        out.status.success(),
        "--help must exit 0, got {:?}",
        out.status
    );

    let stdout = stdout(&out);
    // The correctness bar: help shows real defaults, not `None`.
    assert!(stdout.contains("[default: 3000]"), "{stdout}");
    assert!(stdout.contains("[default: 127.0.0.1]"), "{stdout}");
}

#[test]
fn version_goes_to_stdout_and_exits_zero() {
    let out = run(&["--version"], &[]);
    assert!(out.status.success(), "--version must exit 0");
    assert!(stdout(&out).contains("1.2.3"), "{}", stdout(&out));
}

/// A bad flag is clap's to report: its own diagnostic, and a non-zero exit.
#[test]
fn a_bad_flag_is_reported_by_clap_and_exits_non_zero() {
    let out = run(&["--port", "banana"], &[]);
    assert!(!out.status.success(), "a bad flag must exit non-zero");

    let stderr = stderr(&out);
    assert!(stderr.contains("invalid value"), "{stderr}");
    assert!(stderr.contains("--port"), "{stderr}");
}

#[test]
fn an_unknown_flag_is_reported_by_clap_and_exits_non_zero() {
    let out = run(&["--nonexistent"], &[]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("unexpected argument"),
        "{}",
        stderr(&out)
    );
}

/// A bad *environment* value is ours to report. It must name the variable, and
/// must not quietly fall back to the default.
#[test]
fn a_bad_environment_value_is_attributed_and_exits_non_zero() {
    let out = run(&[], &[("MYAPP_PORT", "banana")]);
    assert!(!out.status.success(), "a bad env value must not be ignored");

    let stderr = stderr(&out);
    assert!(
        stderr.contains("invalid value 'banana' for 'port'"),
        "{stderr}"
    );
    assert!(
        stderr.contains("environment variable MYAPP_PORT"),
        "{stderr}"
    );

    assert!(
        !stdout(&out).contains("port=3000"),
        "must not fall back to the default: {}",
        stdout(&out)
    );
}

/// The error must arrive as a readable message, not a `Debug` dump. `?` in
/// `main` would print the latter, which is why the docs recommend otherwise.
#[test]
fn errors_are_displayed_not_debug_printed() {
    let out = run(&[], &[("MYAPP_PORT", "banana")]);
    let stderr = stderr(&out);

    assert!(
        stderr.starts_with("configuration error: invalid value"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("Invalid {"),
        "stderr shows the Debug representation: {stderr}"
    );
}
