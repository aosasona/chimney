use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    net::{IpAddr, Ipv4Addr},
    path::Path,
};

use crate::error::ChimneyError;

/// Represents the available log levels
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[default]
    Info,
    Debug,
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
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

/// The core configuration options available
#[derive(Debug, Deserialize)]
pub struct Config {
    /// The hostname or IP address to bind the server to (default: 0.0.0.0)
    #[serde(default = "Config::default_host")]
    pub host: IpAddr,

    /// The port number to bind the server to (default: 8080)
    #[serde(default = "Config::default_port")]
    pub port: u16,

    /// The directories to look for sites in (default: "<current directory>/sites")
    #[serde(default = "Config::default_sites_dir")]
    pub sites_directory: Vec<String>,

    /// The log level to use (default: "info")
    #[serde(default)]
    pub log_level: LogLevel,

    /// The various site configurations
    #[serde(skip_deserializing)]
    pub sites: Vec<(String, Site)>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: Config::default_host(),
            port: Config::default_port(),
            sites_directory: Config::default_sites_dir(),
            log_level: LogLevel::Info,
            sites: Vec::new(),
        }
    }
}

impl Config {
    pub fn default_host() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
    }

    pub fn default_port() -> u16 {
        8080
    }

    pub fn default_sites_dir() -> Vec<String> {
        // NOTE: there are cases where this can fail but the changes of hitting either are rare, so
        // we should be fine here
        let cwd = std::env::current_dir().unwrap_or(Path::new(".").to_path_buf());
        let sites_path = cwd.join("sites");
        vec![sites_path.to_string_lossy().to_string()]
    }
}

/// Represents the HTTPS configuration options
#[derive(Debug, Deserialize)]
pub struct HttpsConfig {
    /// Whether HTTPS is enabled or not
    pub enabled: bool,

    /// The path to the SSL certificate file
    pub cert_file: String,

    /// The path to the SSL key file
    pub key_file: String,

    /// The path to the CA bundle file (optional)
    pub ca_bundle_file: Option<String>,
}

/// Represents a site configuration
///
/// A site configuration could be:
/// - defined as part of the root configuration
/// - defined as a separate site configuration file
///
/// This makes it possible to update each site configuration independently or as part of a larger configuration update.
#[derive(Debug, Deserialize)]
pub struct Site {
    /// Whether the site is enabled or not
    #[serde(default = "Site::default_enabled")]
    pub enabled: bool,

    /// The name of the site
    pub name: String,

    /// The root directory of the site (default: ".")
    #[serde(default = "Site::default_root_directory")]
    pub root_directory: String,

    /// The domain names that the site responds to
    pub domain_names: Vec<String>,

    /// The file to fallback to if no other file is found (default: "index.html" for SPAs and
    /// None for other sites)
    pub fallback: Option<String>,

    /// The HTTPS configuration for the site
    pub https_config: Option<HttpsConfig>,

    /// The list of extra headers to include in the response
    /// Variables can be used here to fill in values dynamically from the request or the environment itself
    pub response_headers: Option<Vec<(String, String)>>,

    /// A redirects mapping that maps a source path to a destination path
    /// A redirect is a permanent or temporary redirect from one URL to another, this makes proper
    /// use of the HTTP status codes and conforms to the HTTP standards.
    ///
    /// For example, a request to `/old-path` can be redirected to `/new-path` with a 301 or 302 status code.
    pub redirects: Option<Vec<(String, String)>>,

    /// A rewrites mapping that maps a source path to a destination path
    /// A rewrite is a way to change the target of a request without changing the source URL behind the scenes.
    ///
    /// For example, a request to `/old-path` can be rewritten to `/new-path` without the client knowing about it.
    pub rewrites: Option<Vec<(String, String)>>,
}

impl Site {
    pub fn default_enabled() -> bool {
        true
    }

    pub fn default_root_directory() -> String {
        ".".to_string()
    }

    ///  Constructs a `Site` from a string representation
    pub fn from_string(name: String, input: String) -> Result<Self, ChimneyError> {
        let site: Site = toml::from_str(&input).map_err(|e| ChimneyError::ParseError {
            field: format!("sites.{}", name),
            message: format!("Failed to parse site `{}`: {}", name, e),
        })?;

        // Ensure the site has a name
        if site.name.is_empty() {
            return Err(ChimneyError::ConfigError {
                field: format!("sites.{}", name),
                message: "Site name cannot be empty".to_string(),
            });
        }

        Ok(site)
    }
}
