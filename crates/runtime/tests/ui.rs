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
    // Deliberately value-based, not `is_some()`: CI sets this variable to an
    // empty string on the jobs that *should* run the tests, and an empty-but-set
    // variable must not silently skip them.
    let skip = std::env::var("CLAP_LAYERS_SKIP_UI_TESTS").is_ok_and(|v| !v.is_empty() && v != "0");
    if skip {
        eprintln!("skipping UI tests: CLAP_LAYERS_SKIP_UI_TESTS is set");
        return;
    }

    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
