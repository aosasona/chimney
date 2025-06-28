use std::collections::HashMap;

use log::debug;
use serde::{Deserialize, Serialize};
use toml::Table;

use crate::{error::ChimneyError, with_leading_slash};

use super::{Domain, DomainIndex};

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

/// Represents a redirect rule found for a path
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum RedirectRule {
    /// A redirect rule with a target URL
    Target(String),

    /// A redirect rule with a configuration
    Config {
        /// The target URL or path to redirect to
        to: String,

        /// Whether the redirect is a temporary redirect
        #[serde(default = "RedirectRule::default_temporary_redirect")]
        temporary: bool,

        /// Whether the redirect should be replayed (e.g., for logging or analytics)
        /// A replayed redirect means that the method of the request is preserved
        /// and the request is not changed to a GET request.
        ///
        /// This could be dangerous if the target URL is not safe to replay, for example, posts to
        /// a form or other actions that change state.
        #[serde(default = "RedirectRule::default_replay")]
        replay: bool,
    },
}

impl RedirectRule {
    /// Constructs a new `RedirectRule` with a target URL or path
    pub fn new(to: String, temporary: bool, replay: bool) -> Self {
        RedirectRule::Config {
            to,
            temporary,
            replay,
        }
    }

    pub fn default_temporary_redirect() -> bool {
        false
    }

    pub fn default_replay() -> bool {
        false
    }

    /// Whether the redirect rule is a replay
    pub fn is_replay(&self) -> bool {
        match self {
            RedirectRule::Target(_) => false,
            RedirectRule::Config { replay, .. } => *replay,
        }
    }

    /// Whether the redirect rule is a temporary redirect
    pub fn is_temporary(&self) -> bool {
        match self {
            RedirectRule::Target(_) => false,
            RedirectRule::Config { temporary, .. } => *temporary,
        }
    }

    /// Returns the target URL or path of the redirect rule
    pub fn target(&self) -> String {
        match self {
            RedirectRule::Target(target) => target.clone(),
            RedirectRule::Config { to, .. } => to.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
/// Represents a rewrite rule found for a path
pub enum RewriteRule {
    /// A rewrite rule with a target URL
    Target(String),
}

impl RewriteRule {
    /// Constructs a new `RewriteRule` with a proper leading slash
    pub fn new(to: String) -> Self {
        RewriteRule::Target(to)
    }

    /// Returns the target URL or path of the rewrite rule
    pub fn target(&self) -> String {
        match self {
            RewriteRule::Target(target) => target.to_string(),
        }
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
    pub response_headers: HashMap<String, String>,

    /// A redirects mapping that maps a source path to a destination path
    /// A redirect is a permanent or temporary redirect from one URL to another, this makes proper
    /// use of the HTTP status codes and conforms to the HTTP standards.
    ///
    /// For example, a request to `/old-path` can be redirected to `/new-path`
    #[serde(default)]
    pub redirects: HashMap<String, RedirectRule>,

    /// A rewrites mapping that maps a source path to a destination path
    /// A rewrite is a way to change the target of a request without changing the source URL behind the scenes.
    ///
    /// For example, a request to `/old-path` can be rewritten to `/new-path` without the client knowing about it.
    #[serde(default)]
    pub rewrites: HashMap<String, RewriteRule>,
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
            field: format!("sites.{name}"),
            message: format!("Failed to parse site `{name}`: {e}"),
        })?;

        // Construct the site from the parsed table
        Self::from_table(name, table)
    }

    ///  Constructs a `Site` from a TOML table
    pub fn from_table(name: String, table: Table) -> Result<Self, ChimneyError> {
        let mut site: Self = table.try_into().map_err(|e| ChimneyError::ParseError {
            field: format!("sites.{name}"),
            message: format!("Failed to parse site `{name}`: {e}"),
        })?;

        site.name = name.clone();

        // Ensure the site has a name
        if site.name.is_empty() {
            return Err(ChimneyError::ConfigError {
                field: format!("sites.{name}"),
                message: "Site name cannot be empty".to_string(),
            });
        }

        Ok(site)
    }
}

impl Site {
    /// Finds a redirect rule for a given path
    pub fn find_redirect_rule(&self, path: &str) -> Option<RedirectRule> {
        debug!("Finding redirect for path: {path}");

        if path.is_empty() {
            debug!("Path is empty, cannot find redirect rule");
            return None;
        }

        if self.redirects.is_empty() {
            debug!("No redirects configured for site: {}", self.name);
            return None;
        }

        let redirect_key = with_leading_slash!(path);

        #[cfg(debug_assertions)]
        {
            assert!(!redirect_key.is_empty(), "Redirect key cannot be empty");
            assert!(
                redirect_key.starts_with('/'),
                "Redirect key must start with a leading slash"
            );
        }

        debug!("Looking for redirect key: {redirect_key}");
        match self.redirects.get(&redirect_key) {
            Some(rule) => {
                debug!("Found redirect rule for path: {path}, rule: {rule:?}");
                Some(rule.clone())
            }
            _ => {
                debug!("No redirect found for path: {path}");
                None
            }
        }
    }

    pub fn find_rewrite_rule(&self, path: &str) -> Option<RewriteRule> {
        debug!("Finding rewrite for path: {path}");
        if path.is_empty() {
            debug!("Path is empty, cannot find rewrite rule");
            return None;
        }

        if self.rewrites.is_empty() {
            debug!("No rewrites configured for site: {}", self.name);
            return None;
        }

        let rewrite_key = with_leading_slash!(path);
        #[cfg(debug_assertions)]
        {
            assert!(!rewrite_key.is_empty(), "Rewrite key cannot be empty");
            assert!(
                rewrite_key.starts_with('/'),
                "Rewrite key must start with a leading slash"
            );
        }

        debug!("Looking for rewrite key: {rewrite_key}");
        match self.rewrites.get(&rewrite_key) {
            Some(rule) => {
                debug!("Found rewrite rule for path: {path}, rule: {rule:?}");
                Some(rule.clone())
            }
            _ => {
                debug!("No rewrite found for path: {path}");
                None
            }
        }
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct Sites {
    /// The list of sites in the configuration
    inner: HashMap<String, Site>,

    /// A precompiled index of domain names to site names for fast lookups
    #[serde(skip_serializing, skip_deserializing)]
    domain_index: DomainIndex,
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
            domain_index: DomainIndex::default(),
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
    pub fn get(&self, name: &str) -> Option<&Site> {
        self.inner.iter().find_map(
            |(site_name, site)| {
                if site_name == name { Some(site) } else { None }
            },
        )
    }

    /// Adds a site configuration to the config
    pub fn add(&mut self, site: Site) -> Result<(), ChimneyError> {
        if self.get(&site.name).is_some() {
            return Err(ChimneyError::ConfigError {
                field: format!("sites.{}", site.name),
                message: "Site with this name already exists".to_string(),
            });
        }

        let site_clone = site.clone();
        self.inner.insert(site.name.clone(), site);
        self.rebuild_site_index(&site_clone)?;

        Ok(())
    }

    /// Updates an existing site configuration in the config
    pub fn update(&mut self, site: Site) -> Result<(), ChimneyError> {
        if self.get(&site.name).is_none() {
            return Err(ChimneyError::ConfigError {
                field: format!("sites.{}", site.name),
                message: "Site with this name does not exist".to_string(),
            });
        }

        let site_clone = site.clone();
        self.inner.insert(site.name.clone(), site);
        self.rebuild_site_index(&site_clone)?;

        Ok(())
    }

    /// Removes a site configuration from the config
    pub fn remove(&mut self, name: &str) -> Result<(), ChimneyError> {
        if self.inner.remove(name).is_some() {
            self.domain_index.clear_for_site(name);
            return Ok(());
        }

        Err(ChimneyError::ConfigError {
            field: format!("sites.{name}"),
            message: "Site with this name does not exist".to_string(),
        })
    }

    /// Returns an iterator over the site configurations
    pub fn values(&self) -> impl Iterator<Item = &Site> {
        self.inner.values()
    }

    /// Finds a site configuration by its domain/host name
    pub fn find_by_hostname(&self, domain: &str) -> Option<&Site> {
        let domain: Domain = Domain::try_from(domain.to_string())
            .map_err(|e| ChimneyError::DomainParseError(e.to_string()))
            .ok()?;

        let site_name = self.domain_index.get(&domain);
        match site_name {
            Some(name) => self.inner.get(name),
            None => None,
        }
    }

    /// Rebuilds the domain index for a particular site
    /// All existing domains for that site would be removed and then re-added with the provided
    /// site as the source of truth
    fn rebuild_site_index(&mut self, site: &Site) -> Result<(), ChimneyError> {
        debug!("Rebuilding index for site with empty name, skipping");

        // We don't allow empty site names since we need it in the index
        if site.name.is_empty() {
            debug!("Site name is empty, skipping index rebuild");
            return Err(ChimneyError::ConfigError {
                field: "sites".to_string(),
                message: "Site name cannot be empty".to_string(),
            });
        }

        // To get rid of domains that might have been removed, changed or renamed, we clear the index
        debug!("Clearing domain index for site: {}", site.name);
        self.domain_index.clear_for_site(&site.name);

        // Now we can re-add all domains for this site
        for domain in &site.domain_names {
            debug!("Adding domain '{}' for site '{}'", domain, site.name);

            let domain = Domain::try_from(domain.clone())
                .map_err(|e| ChimneyError::DomainParseError(e.to_string()))?;
            self.domain_index.insert(domain, site.name.clone())?;
        }

        debug!("Rebuilt index for site: {}", site.name);
        Ok(())
    }
}
