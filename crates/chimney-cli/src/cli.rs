use std::{path::PathBuf, sync::Arc, time::Duration};

use chimney::{
    config::{self, Config, Format, LogLevel, Site},
    config_log_debug, config_log_warn, filesystem,
    server::Server,
    tls::{CertRequestOptions, LETS_ENCRYPT_PRODUCTION_URL, LETS_ENCRYPT_STAGING_URL},
};
use clap::{Parser, Subcommand};

use crate::{
    error::{self, CliError},
    format::FormatType,
};

/// A constant array of default configuration file paths to use if none is provided.
const DEFAULT_CONFIG_DIRS: [&str; 4] = [
    "/etc/chimney/config.toml",
    "~/.config/chimney.toml",
    "~/.config/chimney/chimney.toml",
    "chimney.toml",
];

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the server with the provided configuration
    Serve {
        /// Path to the configuration file
        #[arg(
            short,
            long = "config",
            alias = "config-path",
            help = "Path to the Chimney configuration file"
        )]
        config: Option<String>,
    },

    /// Create a new chimney configuration file in the target directory
    #[command(
        arg_required_else_help = true,
        about = "Generate a default Chimney configuration file"
    )]
    Init {
        /// The path to the target directory where the configuration file will be created
        #[arg(
            short,
            long,
            required = false,
            default_value = ".",
            help = "Target path to create the Chimney configuration file in"
        )]
        path: PathBuf,

        /// The format of the configuration file to generate
        /// Possible values: `toml`
        /// Default value: `toml`
        #[arg(
            short,
            long,
            required = false,
            default_value = "toml",
            help = "The format of the configuration file to generate"
        )]
        format: FormatType,
    },

    /// Print the version of the Chimney CLI
    #[command(about = "Print the version of the Chimney CLI")]
    Version,

    /// Request a TLS certificate for domains via ACME (Let's Encrypt)
    ///
    /// This command requests a certificate using the ACME protocol with TLS-ALPN-01 validation.
    /// The server must be able to receive connections on the challenge port (default: 443).
    #[command(
        name = "request-cert",
        about = "Request a TLS certificate for domains via ACME"
    )]
    RequestCert {
        /// Path to the configuration file (to validate site exists)
        #[arg(
            short,
            long = "config",
            alias = "config-path",
            help = "Path to the Chimney configuration file"
        )]
        config: Option<String>,

        /// Name of the site to request certificate for (must exist in config)
        #[arg(
            short = 's',
            long,
            required = true,
            help = "Name of the site to request certificate for (must match a site in your config)"
        )]
        site_name: String,

        /// Domain name(s) to request certificate for (can be specified multiple times)
        #[arg(
            short,
            long = "domain",
            required = true,
            num_args = 1..,
            help = "Domain name(s) to request certificate for"
        )]
        domains: Vec<String>,

        /// Email address for ACME account registration (falls back to config if not provided)
        #[arg(short, long, help = "Email address for ACME account (uses config value if not provided)")]
        email: Option<String>,

        /// Directory to store certificates (falls back to config if not provided)
        #[arg(
            long,
            help = "Directory to store issued certificates (uses config value if not provided)"
        )]
        cert_dir: Option<PathBuf>,

        /// Port to bind for ACME TLS-ALPN-01 challenge
        #[arg(
            long,
            default_value = "443",
            help = "Port to bind for TLS-ALPN-01 challenge (usually 443)"
        )]
        port: u16,

        /// Timeout in seconds for certificate issuance
        #[arg(
            long,
            default_value = "300",
            help = "Timeout in seconds for certificate issuance"
        )]
        timeout: u64,

        /// Use Let's Encrypt staging environment (for testing)
        ///
        /// Staging certificates are not trusted by browsers but allow unlimited
        /// requests, making them ideal for testing.
        #[arg(
            long,
            default_value = "false",
            help = "Use Let's Encrypt staging environment"
        )]
        staging: bool,
    },
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    /// The log level for the application
    log_level: Option<LogLevel>,

    #[clap(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// Creates a new instance of the CLI struct, initializing the CLI parser and setting the log level.
    pub fn new() -> Self {
        // Init CLI parser
        Cli::parse()
    }

    /// Set the log level for the application based on the CLI argument.
    // NOTE: the global log level would ALWAYS override the log level set in the configuration file
    fn set_log_level(&self, configured_log_level: Option<LogLevel>) {
        let log_level = self
            .log_level
            .clone()
            .unwrap_or(configured_log_level.unwrap_or_default())
            .to_log_level_filter();

        env_logger::Builder::new().filter_level(log_level).init();
    }

    /// Execute the CLI command based on the parsed arguments.
    pub async fn execute(&self) -> Result<(), error::CliError> {
        match &self.command {
            Commands::Serve { config } => {
                let config = self.load_config(config)?;

                let config_log_level = config.log_level.clone();
                self.set_log_level(config_log_level);

                log::info!("Parsed configuration: {config:?}");

                self.run_server(config).await
            }
            Commands::Init { path, format } => {
                self.set_log_level(self.log_level.clone());
                self.generate_default_config(path.clone(), format)
            }
            Commands::Version => {
                println!("Chimney CLI version: {}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
            Commands::RequestCert {
                config,
                site_name,
                domains,
                email,
                cert_dir,
                port,
                timeout,
                staging,
            } => {
                self.set_log_level(self.log_level.clone());

                // Load config and validate site exists
                let loaded_config = self.load_config(config)?;
                if loaded_config.sites.get(site_name).is_none() {
                    return Err(CliError::Generic(format!(
                        "Site '{}' not found in configuration. Available sites: {:?}",
                        site_name,
                        loaded_config.sites.into_iter().map(|(name, _)| name).collect::<Vec<_>>()
                    )));
                }

                // Get ACME email from CLI args or fall back to config
                let acme_email = email.clone().or_else(|| {
                    loaded_config.https.as_ref().and_then(|h| h.acme_email.clone())
                }).ok_or_else(|| {
                    CliError::Generic(
                        "ACME email is required. Provide --email or set https.acme_email in config.".to_string()
                    )
                })?;

                // Get cert directory from CLI args or fall back to config
                let cert_dir = cert_dir.clone().unwrap_or_else(|| {
                    loaded_config
                        .https
                        .as_ref()
                        .map(|h| h.cache_directory.clone())
                        .unwrap_or_else(|| PathBuf::from(".chimney/certs"))
                });

                // Determine directory URL based on staging flag
                let directory_url = if *staging {
                    log::info!("Using Let's Encrypt staging environment");
                    LETS_ENCRYPT_STAGING_URL.to_string()
                } else {
                    log::info!("Using Let's Encrypt production environment");
                    LETS_ENCRYPT_PRODUCTION_URL.to_string()
                };

                // Create or resolve cert directory
                std::fs::create_dir_all(&cert_dir).map_err(|e| {
                    CliError::Generic(format!("Failed to create cert directory: {e}"))
                })?;
                let cert_dir = cert_dir.canonicalize().map_err(|e| {
                    CliError::Generic(format!("Failed to resolve cert directory: {e}"))
                })?;

                let options = CertRequestOptions {
                    site_name: site_name.clone(),
                    domains: domains.clone(),
                    email: acme_email,
                    directory_url,
                    cache_dir: cert_dir,
                    challenge_port: *port,
                    timeout: Duration::from_secs(*timeout),
                    ..Default::default()
                };

                self.request_certificate(options, *staging).await
            }
        }
    }

    /// Run the Chimney server with the provided configuration.
    async fn run_server(&self, config: Config) -> Result<(), error::CliError> {
        let fs = filesystem::local::LocalFS::new(PathBuf::from(config.sites_directory.clone()))
            .map_err(CliError::Filesystem)?;

        // Use new_with_tls to enable automatic TLS support
        let server = Server::new_with_tls(Arc::new(fs), config.into())
            .await
            .map_err(|e| CliError::Generic(format!("Failed to create server: {e}")))?;

        // Start the server
        server
            .run()
            .await
            .map_err(|e| CliError::Generic(format!("Failed to start the server: {e}")))?;

        Ok(())
    }

    /// Load the chimney configuration from the specified file path.
    /// If no path is provided, it returns the default configuration.
    fn load_config(&self, config_path: &Option<String>) -> Result<Config, error::CliError> {
        match config_path {
            Some(path) if path.is_empty() => {
                config_log_debug!(
                    "chimney_cli::cli",
                    "Empty configuration path provided, using default configuration."
                );
                Ok(Config::default())
            }
            Some(path) => {
                let path = PathBuf::from(path);
                self.load_config_from_path(path)
            }
            None => {
                // Check default configuration directories
                for dir in DEFAULT_CONFIG_DIRS.iter() {
                    let path = PathBuf::from(dir);
                    if path.exists() && path.is_file() {
                        return self.load_config_from_path(path);
                    }
                }

                config_log_debug!(
                    "chimney_cli::cli",
                    "No configuration path provided, not found in default directories, using default configuration."
                );
                Ok(Config::default())
            }
        }
    }

    fn load_config_from_path(&self, path: PathBuf) -> Result<Config, error::CliError> {
        let path = path
            .canonicalize()
            .map_err(|e| CliError::Generic(format!("Failed to canonicalize path: {e}")))?;

        config_log_debug!(
            "chimney_cli::cli",
            "Loading configuration from: {}",
            path.display()
        );

        if !path.exists() {
            return Err(CliError::Generic(format!(
                "Configuration file does not exist: {}",
                path.display()
            )));
        } else if !path.is_file() {
            return Err(CliError::Generic(format!(
                "Provided path is not a file: {}",
                path.display()
            )));
        }

        let config_content = std::fs::read_to_string(&path).map_err(CliError::Read)?;

        let mut config = config::toml::Toml::from(config_content.as_str())
            .parse()
            .map_err(CliError::Chimney)?;

        self.load_sites_configurations(&mut config)?;

        return Ok(config);
    }

    /// Load the configurations for sites not already defined in the Chimney configuration.
    fn load_sites_configurations(&self, config: &mut Config) -> Result<(), error::CliError> {
        let root = PathBuf::from(&config.sites_directory);
        if !root.exists() {
            config_log_warn!(
                "chimney_cli::cli",
                "Sites directory does not exist: {}, creating it.",
                root.display()
            );
            return Ok(());
        }

        if !root.is_dir() {
            return Err(CliError::Generic(format!(
                "Sites directory is not a directory: {}",
                root.display()
            )));
        }

        let loaded_sites = config
            .sites
            .into_iter()
            .map(|(name, _)| name.to_string())
            .collect::<Vec<_>>();

        for entry in std::fs::read_dir(&root).map_err(CliError::Read)? {
            let entry = entry.map_err(CliError::Read)?;
            let path = entry.path();
            let site_name = entry.file_name().to_string_lossy().to_string();

            // Skip if the entry is not a directory or is already defined in the config
            if !path.is_dir() || loaded_sites.contains(&site_name) {
                continue;
            }

            // We need to read whatever config file they have as a Site
            let config_file = path.join("chimney.toml");
            if !config_file.exists() {
                config_log_warn!(
                    "chimney_cli::cli",
                    "No Chimney configuration file found for site: {site_name}, skipping."
                );
                continue;
            }

            let config_content = std::fs::read_to_string(&config_file).map_err(CliError::Read)?;
            let site_config = Site::from_string(site_name.clone(), &config_content)?;

            // Validate the site's root path doesn't escape sites_directory
            // Note: The actual path resolution happens in chimney-core's Service
            let site_root = path
                .canonicalize()
                .map_err(|e| CliError::Generic(format!("Failed to canonicalize site path: {e}")))?;

            let full_root = site_root.join(&site_config.root);
            let canonical_full_root = full_root.canonicalize().map_err(|e| {
                CliError::Generic(format!("Invalid root path for site {site_name}: {e}"))
            })?;

            let canonical_sites_dir = PathBuf::from(&config.sites_directory)
                .canonicalize()
                .map_err(|e| {
                    CliError::Generic(format!("Failed to resolve sites directory: {e}"))
                })?;

            if !canonical_full_root.starts_with(&canonical_sites_dir) {
                return Err(CliError::Generic(format!(
                    "Site '{}' root path escapes sites directory: {}",
                    site_name,
                    canonical_full_root.display()
                )));
            }

            // Add the site configuration without preprocessing the root path
            // chimney-core will resolve site.root relative to sites_directory
            config_log_debug!(
                "chimney_cli::cli",
                "Adding site configuration for: {site_name}"
            );
            config.sites.add(site_config)?;
        }

        Ok(())
    }

    /// Generate a default Chimney configuration file in the specified target directory.
    fn generate_default_config(&self, path: PathBuf, format: &FormatType) -> Result<(), CliError> {
        let config = Config::default();
        let mut path = path
            .canonicalize()
            .map_err(|e| CliError::Generic(format!("Failed to canonicalize path: {e}")))?;

        // Create the format instance based on the provided format type
        let format_instance: Box<dyn Format> = format.format(None);

        // Validate the target path
        if path.is_dir() {
            path.push(format!("chimney.{}", format_instance.extension()));
        } else if path.exists() {
            log::warn!("The specified path already exists and will be overwritten.");
        } else {
            // Create the target directory if it does not exist
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        };

        // Write the configuration to a file using the specified format
        config.write_to_file(&path, format_instance)?;

        log::info!(
            "Default Chimney configuration file created at: {}",
            path.display()
        );

        Ok(())
    }

    /// Request a TLS certificate for the specified domains via ACME.
    async fn request_certificate(
        &self,
        options: CertRequestOptions,
        staging: bool,
    ) -> Result<(), CliError> {
        log::info!("Requesting certificate for domains: {:?}", options.domains);
        log::info!(
            "Certificates will be stored in: {}",
            options.cache_dir.display()
        );
        log::info!(
            "Binding to port {} for ACME challenge",
            options.challenge_port
        );

        println!("Requesting certificate for: {:?}", options.domains);
        println!("This may take a few minutes...\n");

        let result = chimney::tls::request_certificate(options)
            .await
            .map_err(|e| CliError::Generic(format!("Certificate request failed: {e}")))?;

        println!("\nCertificate issued successfully!");
        println!("  Certificate: {}", result.certificate.cert.display());
        println!("  Private key: {}", result.certificate.key.display());
        println!("\nDomains: {:?}", result.domains);

        if staging {
            println!("\nNote: This is a staging certificate and will not be trusted by browsers.");
            println!("Run without --staging to get a production certificate.");
        }

        Ok(())
    }
}
