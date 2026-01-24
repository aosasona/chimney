use std::collections::HashMap;

use log::debug;
use serde::{Deserialize, Serialize};
use toml::Table;

use crate::{error::ChimneyError, with_leading_slash};

use super::{Domain, DomainIndex};

/// Per-site HTTPS configuration overrides.
///
/// When global `[https]` is enabled, all sites automatically get HTTPS.
/// This struct allows per-site overrides:
/// - Provide `cert_file` + `key_file` to use manual certificates instead of ACME
/// - Set `auto_redirect = false` to disable HTTP→HTTPS redirect for this site
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Https {
    /// Whether to automatically redirect HTTP requests to HTTPS (default: true)
    #[serde(default = "Https::default_auto_redirect")]
    pub auto_redirect: bool,

    /// The path to the SSL certificate file (for manual mode)
    pub cert_file: Option<String>,

    /// The path to the SSL key file (for manual mode)
    pub key_file: Option<String>,

    /// The path to the CA bundle file (optional, for manual mode)
    pub ca_file: Option<String>,
}

/// ACME configuration extracted from the root Config.
/// This consolidates ACME settings that apply to all sites.
#[derive(Debug, Clone)]
pub struct AcmeConfig {
    pub email: Option<String>,
    pub directory_url: Option<String>,
}

impl AcmeConfig {
    pub fn from_config(config: &crate::config::Config) -> Self {
        let https_config = config.https.as_ref();

        Self {
            email: https_config.and_then(|https| https.acme_email.clone()),
            directory_url: https_config.map(|https| https.acme_directory_url.clone()),
        }
    }
}

impl Https {
    pub fn default_auto_redirect() -> bool {
        true
    }

    /// Returns true if manual certificates are configured
    pub fn is_manual(&self) -> bool {
        self.cert_file.is_some() && self.key_file.is_some()
    }

    /// Validates the per-site HTTPS configuration
    pub fn validate(&self, site_name: &str) -> Result<(), ChimneyError> {
        // Check if only one of cert/key is provided (incomplete manual config)
        if self.cert_file.is_some() != self.key_file.is_some() {
            return Err(ChimneyError::ConfigError {
                field: format!("sites.{site_name}.https_config"),
                message: "Both cert_file and key_file must be provided for manual certificates"
                    .to_string(),
            });
        }

        Ok(())
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
    pub fallback_file: Option<String>,

    /// The default index file to serve when a directory is requested
    ///
    /// For example, if a request is made to `/`, the server will look for this file in the root directory. If it was made to `/about/`, it will look for this file in the `/about/` directory.
    pub default_index_file: Option<String>,

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

    /// Returns the index file to serve when a directory is requested
    pub fn index_file(&self) -> String {
        self.default_index_file
            .clone()
            .unwrap_or_else(|| "index.html".to_string())
    }
}

impl Site {
    /// Sets the root directory for the site
    pub fn set_root_directory(&mut self, root: String) {
        debug!("Setting root directory for site {}: {}", self.name, root);
        self.root = root;
    }

    /// Adds a domain name to the site
    pub fn add_domain_name(&mut self, domain: String) {
        debug!("Adding domain name '{}' to site '{}'", domain, self.name);
        if !self.domain_names.contains(&domain) {
            self.domain_names.push(domain);
        } else {
            debug!(
                "Domain '{}' already exists for site '{}'",
                domain, self.name
            );
        }
    }

    /// Adds a redirect rule to the site
    ///
    /// NOTE: this will not prepend a leading slash to the matcher, so it is up to the caller to ensure that the matcher is properly formatted as the matcher could be a regex instead of a simple route
    pub fn add_redirect_rule(&mut self, matcher: String, rule: RedirectRule) {
        debug!(
            "Adding redirect rule for path '{}' to site '{}': {:?}",
            matcher, self.name, rule
        );
        self.redirects.insert(matcher, rule);
    }

    /// Adds a rewrite rule to the site
    ///
    /// NOTE: this will not prepend a leading slash to the matcher, so it is up to the caller to ensure that the matcher is properly formatted as the matcher could be a regex instead of a simple route
    pub fn add_rewrite_rule(&mut self, matcher: String, rule: RewriteRule) {
        debug!(
            "Adding rewrite rule for path '{}' to site '{}': {:?}",
            matcher, self.name, rule
        );
        self.rewrites.insert(matcher, rule);
    }

    /// Adds a response header to the site
    pub fn add_response_header(&mut self, header: String, value: String) {
        debug!(
            "Adding response header '{}' with value '{}' to site '{}'",
            header, value, self.name
        );
        self.response_headers.insert(header, value);
    }

    /// Removes a response header from the site
    pub fn remove_response_header(&mut self, header: &str) {
        debug!(
            "Removing response header '{}' from site '{}'",
            header, self.name
        );
        if !self.response_headers.contains_key(header) {
            debug!(
                "Response header '{}' not found in site '{}', skipping removal",
                header, self.name
            );
            return;
        }
        self.response_headers.remove(header);
    }

    /// Installs a TLS certificate for this site.
    ///
    /// This updates the site's `https_config` to use the specified certificate
    /// and key files. The certificate will be used when the server starts or
    /// when the configuration is reloaded.
    ///
    /// # Arguments
    /// * `cert_path` - Path to the certificate PEM file
    /// * `key_path` - Path to the private key PEM file
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let mut site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .build();
    ///
    /// site.set_certificate("./certs/cert.pem", "./certs/key.pem");
    ///
    /// assert!(site.https_config.is_some());
    /// ```
    pub fn set_certificate(&mut self, cert_path: impl Into<String>, key_path: impl Into<String>) {
        let cert = cert_path.into();
        let key = key_path.into();
        debug!("Installing certificate for site '{}'", self.name);

        self.https_config = Some(Https {
            auto_redirect: true,
            cert_file: Some(cert),
            key_file: Some(key),
            ca_file: None,
        });
    }

    /// Installs a TLS certificate with a CA bundle for this site.
    ///
    /// Similar to `set_certificate`, but also includes a CA bundle file
    /// for certificate chain verification.
    ///
    /// # Arguments
    /// * `cert_path` - Path to the certificate PEM file
    /// * `key_path` - Path to the private key PEM file
    /// * `ca_path` - Path to the CA bundle PEM file
    pub fn set_certificate_with_ca(
        &mut self,
        cert_path: impl Into<String>,
        key_path: impl Into<String>,
        ca_path: impl Into<String>,
    ) {
        let cert = cert_path.into();
        let key = key_path.into();
        let ca = ca_path.into();
        debug!(
            "Installing certificate with CA for site '{}': cert={}, key={}, ca={}",
            self.name, cert, key, ca
        );
        self.https_config = Some(Https {
            auto_redirect: true,
            cert_file: Some(cert),
            key_file: Some(key),
            ca_file: Some(ca),
        });
    }

    /// Removes the TLS certificate configuration from this site.
    ///
    /// After calling this, the site will use ACME for automatic certificate
    /// issuance (if global HTTPS is enabled).
    pub fn remove_certificate(&mut self) {
        debug!(
            "Removing certificate configuration for site '{}'",
            self.name
        );
        self.https_config = None;
    }

    /// Returns true if this site has a manually configured certificate.
    pub fn has_certificate(&self) -> bool {
        self.https_config
            .as_ref()
            .map(|https| https.is_manual())
            .unwrap_or(false)
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
        debug!("Updating site configuration: {}", site.name);
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
        debug!("Removing site configuration: {name}");
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
        debug!("Getting all site configurations");
        self.inner.values()
    }

    /// Finds a site configuration by its domain/host name
    pub fn find_by_hostname(&self, domain: &str) -> Option<&Site> {
        debug!("Finding site by hostname: {domain}");
        let domain: Domain = Domain::try_from(domain.to_string())
            .map_err(|e| ChimneyError::DomainParseError(e.to_string()))
            .ok()?;
        debug!("Looking up domain: {domain}");

        let site_name = self.domain_index.get(&domain);
        debug!("Found site name: {site_name:?}");

        match site_name {
            Some(name) => self.inner.get(name),
            None => None,
        }
    }

    /// Rebuilds the domain index for a particular site
    /// All existing domains for that site would be removed and then re-added with the provided
    /// site as the source of truth
    fn rebuild_site_index(&mut self, site: &Site) -> Result<(), ChimneyError> {
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

/// A builder for constructing `Site` configurations with a fluent API.
///
/// # Example
///
/// ```
/// use chimney::config::SiteBuilder;
///
/// let site = SiteBuilder::new("my-site")
///     .domain("example.com")
///     .domain("www.example.com")
///     .root("./public")
///     .fallback_file("index.html")
///     .response_header("X-Frame-Options", "DENY")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct SiteBuilder {
    name: String,
    root: String,
    domain_names: Vec<String>,
    fallback_file: Option<String>,
    default_index_file: Option<String>,
    https_config: Option<Https>,
    response_headers: HashMap<String, String>,
    redirects: HashMap<String, RedirectRule>,
    rewrites: HashMap<String, RewriteRule>,
}

impl SiteBuilder {
    /// Creates a new `SiteBuilder` with the given site name.
    ///
    /// The name is required and cannot be changed after creation.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            root: Site::default_root_directory(),
            domain_names: Vec::new(),
            fallback_file: None,
            default_index_file: None,
            https_config: None,
            response_headers: HashMap::new(),
            redirects: HashMap::new(),
            rewrites: HashMap::new(),
        }
    }

    /// Adds a domain name to the site.
    ///
    /// This method is chainable and can be called multiple times.
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .domain("www.example.com")
    ///     .build();
    /// ```
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        let domain = domain.into();
        if !self.domain_names.contains(&domain) {
            self.domain_names.push(domain);
        }
        self
    }

    /// Adds multiple domain names to the site.
    ///
    /// This method is chainable and can be called multiple times.
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domains(["example.com", "www.example.com"])
    ///     .build();
    /// ```
    pub fn domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for domain in domains {
            let domain = domain.into();
            if !self.domain_names.contains(&domain) {
                self.domain_names.push(domain);
            }
        }
        self
    }

    /// Sets the root directory for the site.
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .root("./public")
    ///     .build();
    /// ```
    pub fn root(mut self, root: impl Into<String>) -> Self {
        self.root = root.into();
        self
    }

    /// Sets the fallback file for the site (useful for SPAs).
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .fallback_file("index.html")
    ///     .build();
    /// ```
    pub fn fallback_file(mut self, file: impl Into<String>) -> Self {
        self.fallback_file = Some(file.into());
        self
    }

    /// Sets the default index file for directory requests.
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .default_index_file("index.htm")
    ///     .build();
    /// ```
    pub fn default_index_file(mut self, file: impl Into<String>) -> Self {
        self.default_index_file = Some(file.into());
        self
    }

    /// Sets the HTTPS configuration for the site.
    ///
    /// # Example
    /// ```
    /// use chimney::config::{SiteBuilder, Https};
    ///
    /// let https = Https {
    ///     auto_redirect: true,
    ///     cert_file: Some("cert.pem".to_string()),
    ///     key_file: Some("key.pem".to_string()),
    ///     ca_file: None,
    /// };
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .https(https)
    ///     .build();
    /// ```
    pub fn https(mut self, config: Https) -> Self {
        self.https_config = Some(config);
        self
    }

    /// Sets manual TLS certificate paths for the site.
    ///
    /// This is a convenience method that creates an `Https` config with manual certificates.
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .manual_cert("./certs/cert.pem", "./certs/key.pem")
    ///     .build();
    /// ```
    pub fn manual_cert(
        mut self,
        cert_file: impl Into<String>,
        key_file: impl Into<String>,
    ) -> Self {
        self.https_config = Some(Https {
            auto_redirect: true,
            cert_file: Some(cert_file.into()),
            key_file: Some(key_file.into()),
            ca_file: None,
        });
        self
    }

    /// Adds a response header to the site.
    ///
    /// This method is chainable and can be called multiple times.
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .response_header("X-Frame-Options", "DENY")
    ///     .response_header("X-Content-Type-Options", "nosniff")
    ///     .build();
    /// ```
    pub fn response_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.response_headers.insert(name.into(), value.into());
        self
    }

    /// Adds multiple response headers to the site.
    ///
    /// This method is chainable and can be called multiple times.
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .response_headers([
    ///         ("X-Frame-Options", "DENY"),
    ///         ("X-Content-Type-Options", "nosniff"),
    ///     ])
    ///     .build();
    /// ```
    pub fn response_headers<I, K, V>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (name, value) in headers {
            self.response_headers.insert(name.into(), value.into());
        }
        self
    }

    /// Adds a simple redirect rule to the site.
    ///
    /// This method is chainable and can be called multiple times.
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .redirect("/old-path", "/new-path")
    ///     .build();
    /// ```
    pub fn redirect(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.redirects
            .insert(from.into(), RedirectRule::Target(to.into()));
        self
    }

    /// Adds a redirect rule with full configuration to the site.
    ///
    /// This method is chainable and can be called multiple times.
    ///
    /// # Example
    /// ```
    /// use chimney::config::{SiteBuilder, RedirectRule};
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .redirect_rule("/old", RedirectRule::new("/new".to_string(), true, false))
    ///     .build();
    /// ```
    pub fn redirect_rule(mut self, from: impl Into<String>, rule: RedirectRule) -> Self {
        self.redirects.insert(from.into(), rule);
        self
    }

    /// Adds a rewrite rule to the site.
    ///
    /// This method is chainable and can be called multiple times.
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .rewrite("/api/*", "/backend/api/$1")
    ///     .build();
    /// ```
    pub fn rewrite(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.rewrites
            .insert(from.into(), RewriteRule::Target(to.into()));
        self
    }

    /// Builds the `Site` from the configured options.
    ///
    /// # Example
    /// ```
    /// use chimney::config::SiteBuilder;
    ///
    /// let site = SiteBuilder::new("my-site")
    ///     .domain("example.com")
    ///     .root("./public")
    ///     .build();
    ///
    /// assert_eq!(site.name, "my-site");
    /// assert_eq!(site.domain_names, vec!["example.com"]);
    /// assert_eq!(site.root, "./public");
    /// ```
    pub fn build(self) -> Site {
        Site {
            name: self.name,
            root: self.root,
            domain_names: self.domain_names,
            fallback_file: self.fallback_file,
            default_index_file: self.default_index_file,
            https_config: self.https_config,
            response_headers: self.response_headers,
            redirects: self.redirects,
            rewrites: self.rewrites,
        }
    }
}
