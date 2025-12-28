use std::{path::PathBuf, sync::Arc};

use chimney::{
    config::{self, Config, Format, LogLevel, Site},
    config_log_debug, config_log_warn, filesystem,
    server::Server,
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
                self.set_log_level(None);
                self.generate_default_config(path.clone(), format)
            }
            Commands::Version => {
                println!("Chimney CLI version: {}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
        }
    }

    /// Run the Chimney server with the provided configuration.
    async fn run_server(&self, config: Config) -> Result<(), error::CliError> {
        let fs = filesystem::local::LocalFS::new(PathBuf::from(config.sites_directory.clone()))
            .map_err(CliError::Filesystem)?;

        // Use new_with_tls to enable automatic TLS support
        let server = Server::new_with_tls(Arc::new(fs), Arc::new(config))
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
            let mut site_config = Site::from_string(site_name.clone(), &config_content)?;
            let site_root = path
                .canonicalize()
                .map_err(|e| CliError::Generic(format!("Failed to canonicalize site path: {e}")))?;

            // Now we need to add the site configuration to the main Chimney config
            config_log_debug!(
                "chimney_cli::cli",
                "Adding new site configuration for: {site_name}"
            );

            // Append the site's configured root directory to the canonicalized site path
            // This preserves the "root" setting from the site's chimney.toml
            let full_root = site_root.join(&site_config.root);

            // Validate the path doesn't escape sites_directory
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

            site_config.set_root_directory(canonical_full_root.to_string_lossy().to_string());
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
}
