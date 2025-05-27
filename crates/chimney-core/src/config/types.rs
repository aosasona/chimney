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
pub struct Config {
    /// The hostname or IP address to bind the server to (default: 0.0.0.0)
    pub host: String,

    /// The port number to bind the server to (default: 80)
    pub port: u16,

    /// The directories to look for sites in (default: "<current directory>/sites")
    pub site_directories: Vec<String>,

    /// The log level to use (default: "info")
    pub log_level: LogLevel,

    /// The various site configurations
    pub sites: Vec<Site>,
}

/// Represents the HTTPS configuration options
#[derive(Debug)]
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
#[derive(Debug)]
pub struct Site {
    /// Whether the site is enabled or not
    pub enabled: bool,

    /// The name of the site
    pub name: String,

    /// The root directory of the site (default: ".")
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
