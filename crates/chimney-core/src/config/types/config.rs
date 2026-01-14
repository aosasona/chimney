use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    config::Format,
    error::{ChimneyError, ServerError},
};

use super::{LogLevel, Sites};

pub type ConfigSender = tokio::sync::watch::Sender<Arc<Config>>;
pub type ConfigReceiver = tokio::sync::watch::Receiver<Arc<Config>>;

#[derive(Debug, Clone)]
pub struct ConfigHandle {
    /// The sender for the configuration
    pub sender: ConfigSender,

    /// The receiver for the configuration
    pub receiver: ConfigReceiver,
}

impl ConfigHandle {
    /// Creates a new configuration handle with the given sender and receiver
    pub fn new(sender: ConfigSender, receiver: ConfigReceiver) -> Self {
        ConfigHandle { sender, receiver }
    }

    /// Returns a clone of the current configuration
    pub fn get(&self) -> Arc<Config> {
        self.receiver.borrow().clone()
    }

    pub fn set(&self, config: Config) -> Result<(), ServerError> {
        // Send the new configuration to the receiver
        self.sender
            .send(Arc::new(config))
            .map_err(ServerError::ConfigUpdateFailed)
    }
}

impl From<Config> for ConfigHandle {
    fn from(val: Config) -> Self {
        let (sender, receiver) = tokio::sync::watch::channel(Arc::new(val));
        ConfigHandle::new(sender, receiver)
    }
}

/// Represents the host detection options
/// This is used to determine how the target host i.e. domain or IP address is detected from the
/// request headers
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HostDetectionStrategy {
    /// Automatically detect the host from the request headers
    ///
    /// The result is obtained on the first request and cached for subsequent requests until there
    /// is a need to re-detect it. This will happen if:
    /// - The user-facing proxy changes the header being used or similar
    /// - The server is restarted
    #[default]
    Auto,

    /// A list of headers to check for the host in, in order of precedence
    #[serde(untagged)]
    Manual { target_headers: Vec<String> },
}

impl HostDetectionStrategy {
    /// Returns the default headers to check for the host in (in order of precedence)
    pub fn default_headers() -> Vec<String> {
        vec![
            "Host".to_string(),
            "X-Forwarded-Host".to_string(),
            "X-Forwarded-For".to_string(),
            "X-Real-Host".to_string(),
            "X-Forwarded-Server".to_string(),
        ]
    }

    /// Returns the headers to check for the host in, based on the current configuration
    pub fn target_headers(&self) -> Vec<String> {
        match self {
            HostDetectionStrategy::Auto => Self::default_headers(),
            HostDetectionStrategy::Manual { target_headers } => target_headers.clone(),
        }
    }

    /// Checks if the host detection strategy is set to auto-detect
    pub fn is_auto(&self) -> bool {
        matches!(self, HostDetectionStrategy::Auto)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HttpsConfig {
    /// Whether HTTPS is enabled globally or not (default: true)
    #[serde(default = "HttpsConfig::default_enabled")]
    pub enabled: bool,

    /// The port number to bind the HTTPS server to (default: 8443)
    /// See also `port` for the HTTP port
    #[serde(default = "HttpsConfig::default_port")]
    pub port: u16,

    #[serde(default = "HttpsConfig::default_cache_directory")]
    /// The directory to cache TLS certificates in (for ACME)
    /// Default: ~/.chimney/certs
    pub cache_directory: PathBuf,

    #[serde(default = "HttpsConfig::default_acme_email")]
    /// The email address to use for ACME account registration
    pub acme_email: Option<String>,

    /// The ACME directory URL to use (default: Let's Encrypt production)
    /// See https://letsencrypt.org/docs/acme-endpoint/ for more details
    /// Default: https://acme-v02.api.letsencrypt.org/directory
    #[serde(default = "HttpsConfig::default_acme_directory")]
    pub acme_directory_url: String,
}

impl Default for HttpsConfig {
    fn default() -> Self {
        HttpsConfig {
            enabled: HttpsConfig::default_enabled(),
            port: HttpsConfig::default_port(),
            cache_directory: HttpsConfig::default_cache_directory(),
            acme_email: HttpsConfig::default_acme_email(),
            acme_directory_url: HttpsConfig::default_acme_directory(),
        }
    }
}

impl HttpsConfig {
    pub fn default_enabled() -> bool {
        true
    }

    pub fn default_port() -> u16 {
        8443
    }

    pub fn default_acme_email() -> Option<String> {
        None
    }

    pub fn default_acme_directory() -> String {
        "https://acme-v02.api.letsencrypt.org/directory".to_string()
    }

    pub fn default_cache_directory() -> PathBuf {
        PathBuf::from("~/.chimney/certs")
    }
}

/// The core configuration options available
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// The hostname or IP address to bind the server to (default: 0.0.0.0)
    #[serde(default = "Config::default_host")]
    pub host: IpAddr,

    /// The port number to bind the HTTP server to (default: 8080)
    /// See also `https.port` for the HTTPS port
    #[serde(default = "Config::default_port")]
    pub port: u16,

    /// The HTTPS configuration options (default: enabled on port 8443)
    #[serde(default)]
    pub https: Option<HttpsConfig>,

    /// The host detection options to use (default: "auto")
    #[serde(default)]
    pub host_detection: HostDetectionStrategy,

    /// The directories to look for sites in (default: "<current directory>/sites")
    #[serde(default = "Config::default_sites_dir")]
    pub sites_directory: String,

    /// The log level to use (default: "info")
    #[serde(default)]
    pub log_level: Option<LogLevel>,

    /// The various site configurations
    #[serde(skip_deserializing, skip_serializing_if = "Sites::is_empty")]
    pub sites: Sites,

    /// The actual headers to check for the host in when a request comes in
    /// This serves as a cache for automatic detection
    #[serde(skip_serializing, skip_deserializing)]
    resolved_host_header: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: Config::default_host(),
            port: Config::default_port(),
            https: Some(HttpsConfig::default()),
            host_detection: HostDetectionStrategy::default(),
            sites_directory: Config::default_sites_dir(),
            log_level: Some(LogLevel::default()),
            sites: Sites::default(),
            resolved_host_header: None,
        }
    }
}

// Default implementations for Config
impl Config {
    pub fn default_host() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
    }

    pub fn default_port() -> u16 {
        8080
    }

    pub fn default_https_port() -> Option<u16> {
        Some(8443)
    }

    pub fn default_sites_dir() -> String {
        // NOTE: there are cases where this can fail but the changes of hitting either are rare, so
        // we should be fine here
        let cwd = std::env::current_dir().unwrap_or(Path::new(".").to_path_buf());
        let sites_path = cwd.join("sites");
        sites_path.to_string_lossy().to_string()
    }
}

// IO implementations
impl Config {
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

// Host header resolution implementations
impl Config {
    /// Checks if we already have cached target headers
    pub fn has_resolved_host_header(&self) -> bool {
        self.resolved_host_header.is_some()
    }

    /// Gets the cached target header if it exists
    pub fn resolved_host_header(&self) -> Option<String> {
        self.resolved_host_header.clone()
    }

    /// Sets the cached target header
    pub fn set_resolved_host_header(&mut self, header: String) {
        if header.is_empty() {
            return;
        }

        self.resolved_host_header = Some(header);
    }
}

// TLS certificate directory resolution
impl Config {
    pub fn cert_directory(&self) -> PathBuf {
        if let Some(https_config) = &self.https {
            return https_config.cache_directory.clone();
        }

        PathBuf::from("~/.chimney/certs")
    }
}

// TODO: impelment events
