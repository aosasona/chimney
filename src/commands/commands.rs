use crate::{config, error::ChimneyError, log_info, server::Server};
use clap::{Parser, Subcommand};
use std::{env, path::PathBuf};

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the server
    Run {
        #[arg(
            short,
            long,
            value_name = "CONFIG_PATH",
            default_value = "chimney.toml"
        )]
        config_path: PathBuf,
    },

    /// Create a new chimney configuration file in the target directory
    #[command(arg_required_else_help = true)]
    Init {
        #[arg(value_name = "TARGET_DIR", required = false)]
        target_dir: Option<PathBuf>,
    },
}

#[derive(Parser, Debug)]
pub struct CliOpts {
    #[clap(subcommand)]
    pub command: Commands,
}

pub fn parse_args() -> CliOpts {
    CliOpts::parse()
}

impl CliOpts {
    pub async fn run(&self) -> Result<(), ChimneyError> {
        match &self.command {
            Commands::Run { config_path } => {
                let config = config::read_from_path(&mut config_path.clone())?;
                let server = Server::new(config);
                server.run().await?;
            }
            Commands::Init { target_dir } => {
                let target = match target_dir {
                    Some(s) => s.clone(),
                    None => env::current_dir().map_err(ChimneyError::FailedToGetWorkingDir)?,
                };

                let file_path = config::init_at(&mut target.clone())?;
                log_info!(format!("Created new config file at `{}`", file_path));
            }
        }

        Ok(())
    }
}
