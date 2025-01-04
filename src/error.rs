use crate::resource;
use config::ConfigError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error reading configuration file: {0}")]
    Config(#[from] ConfigError),
    #[error("Error accessing device: {0}")]
    Device(#[from] mikrotik_api::error::Error),
    #[error("Error fetching rows: {0}")]
    Resource(#[from] resource::Error),
}
