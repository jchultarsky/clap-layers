//! Compile-fail tests for the derive's error messages.
//!
//! A derive macro's diagnostics are part of its public UX: the previous
//! implementation silently ignored every malformed `#[layered(...)]` attribute,
//! which turned a typo in `no_env` into a quiet security downgrade. These tests
//! pin the wording so that cannot regress unnoticed.
//!
//! The expected output is rendered by rustc and can shift between toolchains,
//! so CI runs this on stable only and sets `CLAP_LAYERS_SKIP_UI_TESTS` on the
//! MSRV job. Regenerate after intentional changes with:
//!
//! ```bash
//! TRYBUILD=overwrite cargo test -p clap-layers --test ui
//! ```

#[test]
fn ui() {
    // Allowlist, not denylist: only an affirmative value skips. CI sets this to
    // "false" on the jobs that must run these tests, so anything unrecognised —
    // "false", "", a typo — has to mean *run*. A skip condition that fails open
    // silently disables the suite, which is the failure this suite exists to
    // catch.
    let skip = std::env::var("CLAP_LAYERS_SKIP_UI_TESTS")
        .is_ok_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"));
    if skip {
        eprintln!("skipping UI tests: CLAP_LAYERS_SKIP_UI_TESTS is set");
        return;
    }

    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
