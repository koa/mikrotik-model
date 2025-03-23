use crate::model::Resource;
use crate::resource::SentenceResult;
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
pub use mikrotik_model_generator_macro::mikrotik_model;

#[derive(Deserialize, Debug)]
pub struct Credentials {
    pub user: Box<str>,
    pub password: Box<str>,
}

pub type MikrotikDevice = mikrotik_api::prelude::MikrotikDevice<SentenceResult<Resource>>;
