//! # clap-layers
//!
//! A deriving helper for correctly layered configuration in [clap](https://crates.io/crates/clap) applications.
//!
//! ## What Problem Does This Solve?
//!
//! When building CLI applications, users expect configuration to come from multiple sources:
//!
//! - **Explicit flags** you pass on the command line
//! - **Environment variables**
//! - **Configuration files**
//! - **Default values** hard-coded in your struct
//!
//! The order matters: flags should always beat env vars, which beat config files, which beat defaults.
//! The tricky part is detecting whether a value came from an explicit user choice or just a default.
//!
//! `clap-layers` solves this by:
//! 1. Letting you write one struct with all your settings
//! 2. Automatically tracking where each value came from (via clap's `value_source()`)
//! 3. Merging sources in the correct precedence order
//! 4. Giving precise, source-attributed errors when things go wrong

#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[doc(hidden)]
pub use clap_layers_proc::Layered;

pub mod merge;
pub use merge::{parse_env_var, parse_toml_file};

/// Where a configuration value ultimately came from.
///
/// This is useful for debugging which source is providing each value,
/// or for implementing features like `--dump-config`.
///
/// # Examples
///
/// A common pattern is to log the source of each configuration value:
///
/// ```ignore
/// let cfg = Config::layered()?;
///
/// for field in cfg.fields() {
///     println!("{} came from: {}", field.name, field.source);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum SourceLayer {
    /// Value was set via an explicitly passed CLI flag (e.g., `--port 8080`)
    CliFlag(String),
    /// Value was read from an environment variable
    EnvVar(String),
    /// Value was loaded from a configuration file
    ConfigFile(String),
    /// Value came from the struct's default (no external source)
    Default,
}

impl std::fmt::Display for SourceLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceLayer::CliFlag(name) => write!(f, "command-line flag {name}"),
            SourceLayer::EnvVar(var) => write!(f, "environment variable {var}"),
            SourceLayer::ConfigFile(path) => write!(f, "configuration file {path}"),
            SourceLayer::Default => write!(f, "defaults"),
        }
    }
}

/// Errors that can occur when loading configuration.
///
/// Each error variant includes context about where the problem occurred,
/// making it easier to debug configuration issues.
#[derive(thiserror::Error, Debug)]
pub enum LayeredError {
    /// A configuration file could not be read.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use clap_layers::LayeredError;
    ///
    /// // This error includes the path that failed:
    /// let err = LayeredError::Io { ... };
    /// assert!(err.to_string().contains("config.toml"));
    /// ```
    #[error("could not read config file '{path}': {source}")]
    Io {
        /// The path to the file that could not be read.
        path: String,
        /// The underlying I/O error from `std::io`.
        source: std::io::Error,
    },
    /// A configuration file contains invalid TOML syntax or structure.
    ///
    /// # Example
    ///
    /// ```text
    /// // If config.toml has:
    /// //   port = "not-a-number"
    /// //
    /// // You'll get an error like:
    /// //   "TOML parse error at line 1, column 8: invalid value for u16: not-a-number"
    /// ```
    #[error("TOML parse error at {position}: {message}")]
    TomlParse {
        /// The human-readable message from the TOML parser.
        message: String,
        /// The position in the file where the error occurred.
        position: String,
    },
    /// An environment variable contains a value that cannot be parsed for a field.
    ///
    /// # Example
    ///
    /// ```text
    /// // If PORT=not-a-number is set:
    /// //
    /// // Error: "invalid environment variable 'PORT' for 'port': invalid digit found in string"
    /// ```
    #[error("invalid environment variable '{var}' for '{field}': {source}")]
    Env {
        /// The name of the environment variable that failed.
        var: String,
        /// The field that failed to parse.
        field: String,
        /// The underlying error from parsing (e.g., "invalid digit found in string").
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Trait implemented by the `#[derive(Layered)]` macro.
///
/// This trait provides a single method to load configuration from all sources
/// with proper precedence: explicit flags > environment variables > config files > defaults.
///
/// ## Usage
///
/// ```text
/// use clap::Parser;
/// use clap_layers::Layered;
///
/// #[derive(Parser, Layered, Debug)]
/// #[command(version, about)]
/// #[layered(file = "myapp.toml", env_prefix = "MYAPP")]
/// struct Config {
///     #[arg(long, default_value_t = 3000)]
///     port: u16,
///
///     #[arg(short, long, default_value_t = false)]
///     verbose: bool,
/// }
///
/// fn main() -> anyhow::Result<()> {
///     let cfg = Config::layered()?;
///     println!("{cfg:?}");
///     Ok(())
/// }
/// ```
pub trait Layered: Sized {
    /// Parse configuration from all layers: CLI > env > file > default.
    ///
    /// This is the main entry point for loading configuration.
    ///
    /// ## Returns
    ///
    /// - `Ok(Config)` if all sources load successfully and merge cleanly
    /// - `Err(LayeredError)` with details about what went wrong and where
    ///
    /// ## Precedence Order
    ///
    /// 1. **CLI flags** (explicitly passed on command line)
    /// 2. **Environment variables** (with the prefix from `#[layered(env_prefix = "...")]`)
    /// 3. **Config file** (at the path from `#[layered(file = "path/to/file.toml")]`)
    /// 4. **Defaults** (from `#[arg(default_value_t = ...)]` or field initializers)
    ///
    /// ## Example
    ///
    /// ```text
    /// let cfg = match Config::layered() {
    ///     Ok(cfg) => cfg,
    ///     Err(e) => {
    ///         eprintln!("Configuration error: {e}");
    ///         std::process::exit(1);
    ///     }
    /// };
    /// ```
    fn layered() -> Result<Self, LayeredError>;
}
