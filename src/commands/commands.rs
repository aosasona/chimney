use crate::{config, config::Config, error::ChimneyError, server};
use clap::{Parser, Subcommand};
use std::{env, path::PathBuf};

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the server
    Run,

    /// Create a new chimney configuration file in the target directory
    Init {
        #[arg(short, long)]
        target_dir: Option<PathBuf>,
    },
}

#[derive(Parser, Debug)]
pub struct CliOpts {
    #[clap(short, long, default_value = "chimney.toml")]
    pub config_path: PathBuf,

    #[clap(subcommand)]
    pub command: Commands,
}

pub fn parse_args() -> CliOpts {
    CliOpts::parse()
}

impl CliOpts {
    pub fn run(self: &Self, _config: &Config) -> Result<(), ChimneyError> {
        match &self.command {
            Commands::Run => {
                let config = config::read_from_path(&self.config_path)?;
                server::run(config)?;
            }
            Commands::Init { target_dir } => {
                let target = match target_dir {
                    Some(s) => s.clone(),
                    None => env::current_dir()
                        .map_err(|e| ChimneyError::FailedToGetWorkingDir { source: e })?,
                };

                let file_path = config::init_at(&target)?;
                println!("Created new config file at: {:?}", file_path);
            }
        }

        Ok(())
    }
}
