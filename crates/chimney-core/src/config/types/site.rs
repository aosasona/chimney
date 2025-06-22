// TODO: precompile domain -> site mapping to speed up lookups
// TODO: add specialized Domain type to hold the hostname and port

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::error::ChimneyError;

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

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct Sites {
    /// The list of sites in the configuration
    inner: HashMap<String, Site>,
}

impl<'a> IntoIterator for &'a Sites {
    type Item = (&'a String, &'a Site);
    type IntoIter = std::collections::hash_map::Iter<'a, String, Site>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl Sites {
    /// Constructs a `Sites` from a vector of site configurations
    pub fn from_vec(sites: Vec<(String, Site)>) -> Self {
        Self {
            inner: sites.into_iter().collect::<HashMap<_, _>>(),
        }
    }

    /// Checks if there are no sites in the configuration
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of sites in the configuration
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Gets the site configuration by name
    pub fn get_site(&self, name: &str) -> Option<&Site> {
        self.inner.iter().find_map(
            |(site_name, site)| {
                if site_name == name { Some(site) } else { None }
            },
        )
    }

    /// Adds a site configuration to the config
    pub fn add_site(&mut self, site: Site) -> Result<(), ChimneyError> {
        // TODO: pre-compile index here
        if self.get_site(&site.name).is_some() {
            return Err(ChimneyError::ConfigError {
                field: format!("sites.{}", site.name),
                message: "Site with this name already exists".to_string(),
            });
        }
        self.inner.insert(site.name.clone(), site);
        Ok(())
    }

    /// Updates an existing site configuration in the config
    pub fn update_site(&mut self, site: Site) -> Result<(), ChimneyError> {
        // TODO: update index here
        if self.get_site(&site.name).is_none() {
            return Err(ChimneyError::ConfigError {
                field: format!("sites.{}", site.name),
                message: "Site with this name does not exist".to_string(),
            });
        }

        self.inner.insert(site.name.clone(), site);
        Ok(())
    }

    /// Removes a site configuration from the config
    pub fn remove_site(&mut self, name: &str) -> Result<(), ChimneyError> {
        // TODO: update index here too
        if self.inner.remove(name).is_some() {
            return Ok(());
        }

        Err(ChimneyError::ConfigError {
            field: format!("sites.{}", name),
            message: "Site with this name does not exist".to_string(),
        })
    }

    /// Returns an iterator over the site configurations
    pub fn values(&self) -> impl Iterator<Item = &Site> {
        self.inner.values()
    }

    /// Finds a site configuration by its domain/host name
    pub fn find_by_hostname(&self, domain: &str) -> Option<&Site> {
        // TODO: use precompiled reference map
        self.inner.iter().find_map(|(_, site)| {
            if site
                .domain_names
                .iter()
                .any(|d| d.eq_ignore_ascii_case(domain))
            {
                Some(site)
            } else {
                None
            }
        })
    }
}
