use crate::{model::Resource, resource::SentenceResult};
use serde::Deserialize;

pub mod ascii;
pub mod error;
pub mod generator;
pub mod hwconfig;
pub mod model;
pub mod repository;
pub mod resource;
mod util;
pub mod value;
pub use mac_address::MacAddress;
pub use mikrotik_model_generator_macro::mikrotik_model;
pub mod mikrotik_api {
    pub use mikrotik_api::prelude::*;
}

#[derive(Deserialize, Debug)]
pub struct Credentials {
    pub user: Box<str>,
    pub password: Box<str>,
}

pub type MikrotikDevice = mikrotik_api::MikrotikDevice<SentenceResult<Resource>>;
