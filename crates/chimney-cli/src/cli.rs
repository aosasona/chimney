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
    /// The name of the application
    name: String,

    #[arg(short, long, default_value = "info")]
    /// The log level for the application
    log_level: Option<LogLevel>,

    #[clap(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn new() -> Self {
        // Init CLI parser
        let cli = Cli::parse();

        // Set the log level based on the CLI argument
        // NOTE: this will always override the log level set in the configuration file
        let level = cli
            .log_level
            .clone()
            .unwrap_or(LogLevel::Info)
            .to_log_level_filter();
        env_logger::Builder::new().filter_level(level).init();

        cli
    }
}
