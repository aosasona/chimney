use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    net::{IpAddr, Ipv4Addr},
    path::Path,
};
use toml::Table;

use crate::error::ChimneyError;

use super::Format;

/// Represents the available log levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl Default for LogLevel {
    fn default() -> Self {
        if cfg!(debug_assertions) {
            LogLevel::Trace
        } else {
            LogLevel::Info
        }
    }
}

impl LogLevel {
    /// Converts the log level to a `log::LevelFilter`
    pub fn to_log_level_filter(&self) -> log::LevelFilter {
        match self {
            LogLevel::Off => log::LevelFilter::Off,
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }

    /// Checks if the current log level is lower than the provided log level
    pub fn is_higher_than(&self, other: &LogLevel) -> bool {
        self.to_log_level_filter() > other.to_log_level_filter()
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(LogLevel::Off),
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_str = match self {
            LogLevel::Off => "off",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        };
        write!(f, "{}", level_str)
    }
}

/// The core configuration options available
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// The hostname or IP address to bind the server to (default: 0.0.0.0)
    #[serde(default = "Config::default_host")]
    pub host: IpAddr,

    /// The port number to bind the server to (default: 8080)
    #[serde(default = "Config::default_port")]
    pub port: u16,

    /// The directories to look for sites in (default: "<current directory>/sites")
    #[serde(default = "Config::default_sites_dir")]
    pub sites_directory: String,

    /// The log level to use (default: "info")
    #[serde(default)]
    pub log_level: Option<LogLevel>,

    /// The various site configurations
    #[serde(skip_deserializing, skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<(String, Site)>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: Config::default_host(),
            port: Config::default_port(),
            sites_directory: Config::default_sites_dir(),
            log_level: Some(LogLevel::default()),
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

    pub fn default_sites_dir() -> String {
        // NOTE: there are cases where this can fail but the changes of hitting either are rare, so
        // we should be fine here
        let cwd = std::env::current_dir().unwrap_or(Path::new(".").to_path_buf());
        let sites_path = cwd.join("sites");
        sites_path.to_string_lossy().to_string()
    }
}

impl Config {
    /// Gets the site configuration by name
    pub fn get_site(&self, name: &str) -> Option<&Site> {
        self.sites.iter().find_map(
            |(site_name, site)| {
                if site_name == name { Some(site) } else { None }
            },
        )
    }

    /// Adds a site configuration to the config
    pub fn add_site(&mut self, site: Site) -> Result<(), ChimneyError> {
        if self.get_site(&site.name).is_some() {
            return Err(ChimneyError::ConfigError {
                field: format!("sites.{}", site.name),
                message: "Site with this name already exists".to_string(),
            });
        }
        self.sites.push((site.name.clone(), site));
        Ok(())
    }

    /// Updates an existing site configuration in the config
    pub fn update_site(&mut self, site: Site) -> Result<(), ChimneyError> {
        if let Some(pos) = self
            .sites
            .iter()
            .position(|(site_name, _)| site_name == &site.name)
        {
            self.sites[pos] = (site.name.clone(), site);
            Ok(())
        } else {
            Err(ChimneyError::ConfigError {
                field: format!("sites.{}", site.name),
                message: "Site with this name does not exist".to_string(),
            })
        }
    }

    /// Removes a site configuration from the config
    pub fn remove_site(&mut self, name: &str) -> Result<(), ChimneyError> {
        if let Some(pos) = self
            .sites
            .iter()
            .position(|(site_name, _)| site_name == name)
        {
            self.sites.remove(pos);
            Ok(())
        } else {
            Err(ChimneyError::ConfigError {
                field: format!("sites.{}", name),
                message: "Site with this name does not exist".to_string(),
            })
        }
    }

    /// Writes the configuration to a file in the specified format
    pub fn write_to_file<P: AsRef<Path>>(
        &self,
        path: P,
        format: Box<dyn Format<'_>>,
    ) -> Result<(), ChimneyError> {
        // Convert the configuration to a string representation in the specified format
        let config_str = format.to_format_string(self)?;

        // Write the string representation to the file
        std::fs::write(path, config_str).map_err(ChimneyError::IOError)?;

        Ok(())
    }
}

/// Represents the HTTPS configuration options
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Https {
    /// Whether HTTPS is enabled or not
    #[serde(default = "Https::default_enabled")]
    pub enabled: bool,

    /// Whether to automatically issue certificates using Let's Encrypt or similar services
    #[serde(default = "Https::default_auto_issue")]
    pub auto_issue: bool,

    /// Whether to automatically redirect HTTP requests to HTTPS
    #[serde(default = "Https::default_auto_redirect")]
    pub auto_redirect: bool,

    /// The path to the SSL certificate file
    pub cert_file: Option<String>,

    /// The path to the SSL key file
    pub key_file: Option<String>,

    /// The path to the CA bundle file (optional)
    pub ca_file: Option<String>,
}

impl Https {
    pub fn default_enabled() -> bool {
        false
    }

    pub fn default_auto_redirect() -> bool {
        true
    }

    pub fn default_auto_issue() -> bool {
        true
    }
}

/// Represents a site configuration
///
/// A site configuration could be:
/// - defined as part of the root configuration
/// - defined as a separate site configuration file
///
/// This makes it possible to update each site configuration independently or as part of a larger configuration update.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Site {
    /// The name of the site
    #[serde(skip_deserializing)]
    pub name: String,

    /// The root directory of the site (default: ".")
    #[serde(default = "Site::default_root_directory")]
    pub root: String,

    /// The domain names that the site responds to
    pub domain_names: Vec<String>,

    /// The file to fallback to if no other file is found (default: "index.html" for SPAs and
    /// None for other sites)
    pub fallback: Option<String>,

    /// The HTTPS configuration for the site
    pub https_config: Option<Https>,

    /// The list of extra headers to include in the response
    /// Variables can be used here to fill in values dynamically from the request or the environment itself
    #[serde(default)]
    pub response_headers: Vec<(String, String)>,

    /// A redirects mapping that maps a source path to a destination path
    /// A redirect is a permanent or temporary redirect from one URL to another, this makes proper
    /// use of the HTTP status codes and conforms to the HTTP standards.
    ///
    /// For example, a request to `/old-path` can be redirected to `/new-path` with a 301 or 302 status code.
    #[serde(default)]
    pub redirects: Vec<(String, String)>,

    /// A rewrites mapping that maps a source path to a destination path
    /// A rewrite is a way to change the target of a request without changing the source URL behind the scenes.
    ///
    /// For example, a request to `/old-path` can be rewritten to `/new-path` without the client knowing about it.
    #[serde(default)]
    pub rewrites: Vec<(String, String)>,
}

impl Site {
    pub fn default_enabled() -> bool {
        true
    }

    pub fn default_root_directory() -> String {
        ".".to_string()
    }

    /// Constructs a `Site` from a string representation
    pub fn from_string(name: String, input: &str) -> Result<Self, ChimneyError> {
        // Parse the input string as a TOML table
        let table: Table = toml::from_str(input).map_err(|e| ChimneyError::ParseError {
            field: format!("sites.{}", name),
            message: format!("Failed to parse site `{}`: {}", name, e),
        })?;

        // Construct the site from the parsed table
        Self::from_table(name, table)
    }

    ///  Constructs a `Site` from a TOML table
    pub fn from_table(name: String, table: Table) -> Result<Self, ChimneyError> {
        let mut site: Self = table.try_into().map_err(|e| ChimneyError::ParseError {
            field: format!("sites.{}", name),
            message: format!("Failed to parse site `{}`: {}", name, e),
        })?;

        site.name = name.clone();

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
