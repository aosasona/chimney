use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::Path,
};

use crate::{config::Format, error::ChimneyError};

use super::{LogLevel, Site};

/// Represents the host detection options
/// This is used to determine how the target host i.e. domain or IP address is detected from the
/// request headers
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
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
    Headers(Vec<String>),
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
            HostDetectionStrategy::Headers(headers) => headers.clone(),
        }
    }

    /// Checks if the host detection strategy is set to auto-detect
    pub fn is_auto(&self) -> bool {
        matches!(self, HostDetectionStrategy::Auto)
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
    #[serde(skip_deserializing, skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<(String, Site)>,

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
            host_detection: HostDetectionStrategy::default(),
            sites_directory: Config::default_sites_dir(),
            log_level: Some(LogLevel::default()),
            sites: Vec::new(),
            resolved_host_header: None,
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

// TODO: implement target detection logic for config stuff
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
