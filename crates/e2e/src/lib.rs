//! Compile-checks for documentation that lives outside any published crate.
//!
//! `README.md` and `docs/DESIGN.md` sit at the workspace root. Hosting their
//! doctests in `clap-layers` itself would mean an `include_str!` reaching above
//! that crate's own directory — a path that exists in this repository and not in
//! the packaged `.crate`, so `cargo test --doc` on a vendored copy would fail to
//! compile. This crate is `publish = false`, so the paths always resolve.
//!
//! An unchecked example is how the documentation previously came to describe an
//! API that never shipped; the check has to live somewhere.

/// The README's examples, compiled.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
struct Readme;

/// The design document's examples, compiled. It is linked from the README as
/// the starting point for understanding the crate, so it is held to the same
/// standard.
#[cfg(doctest)]
#[doc = include_str!("../../../docs/DESIGN.md")]
struct DesignDoc;
