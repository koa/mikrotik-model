use crate::model::Resource;
use crate::resource::SentenceResult;
use serde::Deserialize;

pub mod ascii;
pub mod config_set;
pub mod error;
pub mod generator;
pub mod hwconfig;
pub mod model;
pub mod repository;
pub mod resource;
mod util;
pub mod value;

#[derive(Deserialize, Debug)]
pub struct Credentials {
    pub user: Box<str>,
    pub password: Box<str>,
}

pub type MikrotikDevice = mikrotik_api::prelude::MikrotikDevice<SentenceResult<Resource>>;
