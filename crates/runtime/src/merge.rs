//! Runtime support for loading and merging configuration sources.
//!
//! This module provides helper functions used by the `#[derive(Layered)]` macro
//! to read configuration from files and environment variables.

use crate::LayeredError;
use std::collections::HashMap;

/// Parse a TOML file into a map of string key-value pairs.
///
/// Currently uses simple key=value parsing. In future versions,
/// full TOML parsing with proper type handling will be implemented using serde.
///
/// # Arguments
///
/// - `path`: The path to the TOML configuration file
///
/// # Returns
///
/// A `HashMap<String, String>` containing the parsed key-value pairs,
/// or a [`LayeredError::Io`] if the file cannot be read.
///
/// # Examples
///
/// ```ignore
/// use clap_layers::merge::parse_toml_file;
///
/// let config = parse_toml_file("myapp.toml")?;
/// assert_eq!(config.get("port"), Some(&"8080".to_string()));
/// ```
pub fn parse_toml_file(path: &str) -> Result<HashMap<String, String>, LayeredError> {
    let content = std::fs::read_to_string(path).map_err(|e| LayeredError::Io {
        path: path.to_string(),
        source: e,
    })?;

    let mut values = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().trim_matches('"').to_string();
            values.insert(key, value);
        }
    }

    Ok(values)
}

/// Parse an environment variable.
///
/// # Arguments
///
/// - `var_name`: The name of the environment variable to read
///
/// # Returns
///
/// The variable's value if it exists, or None if the variable is not set.
pub fn parse_env_var(var_name: &str) -> Option<String> {
    std::env::var(var_name).ok()
}
