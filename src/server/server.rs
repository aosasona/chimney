use crate::{config::Config, error::ChimneyError};

pub fn run(config: Config) -> Result<(), ChimneyError> {
    println!("{:?}", config);
    Ok(())
}
