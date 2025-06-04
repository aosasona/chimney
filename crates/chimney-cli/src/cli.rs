use std::path::PathBuf;

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

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[clap(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn new() -> Self {
        Cli::parse()
    }

    /// Return the debug state of the CLI
    pub fn debug(&self) -> bool {
        self.debug > 0
    }
}
