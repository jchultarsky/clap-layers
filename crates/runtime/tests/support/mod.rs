//! Shared helpers for the integration tests.
//!
//! Lives in a subdirectory so cargo treats it as a module rather than as its
//! own test binary.

// Each test binary compiles this module separately and uses a different subset
// of it, so unused-item warnings here are noise rather than signal.
#![allow(dead_code)]

use clap_layers::Env;
use std::path::{Path, PathBuf};

/// Scratch directory for config files written by tests. Relative to the crate
/// root, which is the working directory for integration tests.
const TMP_DIR: &str = ".test-tmp";

/// An empty environment: the field's env layer is never consulted.
pub(crate) fn no_env() -> Env {
    Env::empty()
}

/// A config file that deletes itself when the test ends.
///
/// A `#[layered(file = "...")]` path is fixed at compile time, so a test's
/// config file is named, shared, mutable state. Two tests using the same name
/// will clobber each other and fail intermittently — so creation is
/// **exclusive**, turning that mistake into an immediate, obvious failure
/// instead of a race. Give every test its own file name.
pub(crate) struct TempToml {
    path: PathBuf,
}

impl TempToml {
    /// Create `.test-tmp/<name>` exclusively and write `content` to it.
    ///
    /// # Panics
    ///
    /// If the file already exists — meaning another test claimed the same name,
    /// or a previous run aborted without unwinding. Delete `.test-tmp/` if it is
    /// the latter.
    pub(crate) fn new(name: &str, content: &str) -> Self {
        std::fs::create_dir_all(TMP_DIR).expect("could not create scratch dir");
        let path = Path::new(TMP_DIR).join(name);

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap_or_else(|e| {
                panic!(
                    "could not exclusively create {}: {e}\n\
                     Two tests are probably sharing this file name; each test needs its own.\n\
                     (If a previous run aborted, delete the {TMP_DIR}/ directory.)",
                    path.display()
                )
            });
        std::io::Write::write_all(&mut file, content.as_bytes())
            .expect("could not write scratch config");

        Self { path }
    }

    /// Replace this file's contents.
    ///
    /// For a test that walks several inputs through one struct, whose file path
    /// is fixed at compile time. Safe because the file is exclusively ours.
    pub(crate) fn rewrite(&self, content: &str) {
        std::fs::write(&self.path, content).expect("could not rewrite scratch config");
    }
}

impl Drop for TempToml {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
