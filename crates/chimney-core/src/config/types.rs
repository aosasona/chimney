use std::fmt::Display;

/// Represents the available log levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl From<&str> for LogLevel {
    fn from(level: &str) -> Self {
        match level.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info, // Default to Info if unrecognized
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// The core configuration options available
#[derive(Debug)]
pub struct Config<'a> {
    /// The hostname or IP address to bind the server to (default: 0.0.0.0)
    pub host: &'a str,

    /// The port number to bind the server to (default: 80)
    pub port: u16,

    /// The directories to look for sites in (default: "<current directory>/sites")
    pub site_directories: [&'a str],
}

/// Represents a site configuration
///
/// A site configuration could be:
/// - defined as part of the root configuration
/// - defined as a separate site configuration file
///
/// This makes it possible to update each site configuration independently or as part of a larger configuration update.
#[derive(Debug)]
pub struct Site<'a> {
    /// The name of the site
    pub name: &'a str,
}
