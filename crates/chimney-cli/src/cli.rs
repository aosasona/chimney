use std::path::PathBuf;

use chimney::config::LogLevel;
use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the server with the provided configuration
    Run {
        /// Path to the configuration file
        #[arg(short, long, default_value = "chimney.toml")]
        config: String,
    },

    /// Create a new chimney configuration file in the target directory
    #[command(arg_required_else_help = true)]
    Init {
        /// The path to the target directory where the configuration file will be created
        #[arg(required = false)]
        target_dir: Option<PathBuf>,
    },

    /// Print the version of the Chimney CLI
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

    pub async fn run(&self) -> Result<(), chimney::error::ChimneyError> {
        // Set the log level based on the CLI argument
        self.set_log_level();

        match &self.command {
            Commands::Run { config } => {
                unimplemented!("Run command is not implemented yet. Config: {}", config);
            }
            Commands::Init { target_dir } => {
                unimplemented!(
                    "Init command is not implemented yet. Target directory: {:?}",
                    target_dir
                );
            }
            Commands::Version => {
                println!("Chimney CLI version: {}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
        }
    }
}
