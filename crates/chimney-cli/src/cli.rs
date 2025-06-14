use std::path::PathBuf;

use chimney::config::{self, Config, Format, LogLevel};
use clap::{Parser, Subcommand};

use crate::{
    error::{self, CliError},
    format::FormatType,
};

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the server with the provided configuration
    Run {
        /// Path to the configuration file
        #[arg(short, long, help = "Path to the Chimney configuration file")]
        config_path: Option<String>,
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
    #[arg(short, long, default_value = "info")]
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
    fn set_log_level(&self) {
        // NOTE: this should ALWAYS override the log level set in the configuration file
        let level = self
            .log_level
            .clone()
            .unwrap_or(LogLevel::Info)
            .to_log_level_filter();

        env_logger::Builder::new().filter_level(level).init();
    }

    /// Execute the CLI command based on the parsed arguments.
    pub async fn execute(&self) -> Result<(), error::CliError> {
        // Set the log level based on the CLI argument
        self.set_log_level();

        match &self.command {
            Commands::Run { config_path } => {
                let config = self.load_config(config_path)?;
                log::info!("Parsed configuration: {:?}", config);
                self.run_server(&config).await
            }
            Commands::Init { path, format } => self.generate_default_config(path.clone(), format),
            Commands::Version => {
                println!("Chimney CLI version: {}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
        }
    }

    /// Run the Chimney server with the provided configuration.
    async fn run_server(&self, config: &Config) -> Result<(), error::CliError> {
        log::info!("{:?}", config);

        Ok(())
    }

    /// Load the chimney configuration from the specified file path.
    /// If no path is provided, it returns the default configuration.
    fn load_config(&self, config_path: &Option<String>) -> Result<Config, error::CliError> {
        if let Some(path) = config_path {
            let path = PathBuf::from(path);
            let path = path
                .canonicalize()
                .map_err(|e| CliError::Generic(format!("Failed to canonicalize path: {}", e)))?;

            log::info!("Loading configuration from: {}", path.display());

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
            return config::toml::Toml::from(config_content.as_str())
                .parse()
                .map_err(CliError::Chimney);
        }

        log::info!("No configuration file provided, using default configuration.");
        Ok(Config::default())
    }

    /// Generate a default Chimney configuration file in the specified target directory.
    fn generate_default_config(&self, path: PathBuf, format: &FormatType) -> Result<(), CliError> {
        let config = Config::default();
        let mut path = path
            .canonicalize()
            .map_err(|e| CliError::Generic(format!("Failed to canonicalize path: {}", e)))?;

        // Create the format instance based on the provided format type
        let format_instance: Box<dyn Format> = format.format("");

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
