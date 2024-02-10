use error::ChimneyError;

mod commands;
mod config;
mod error;
mod server;

fn main() -> Result<(), ChimneyError> {
    let command = commands::parse_args();
    let config = config::read_from_path(&command.config_path)?;
    command.run(&config)?;

    Ok(())
}
