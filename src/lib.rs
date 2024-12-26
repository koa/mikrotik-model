use serde::Deserialize;

pub mod error;
pub mod generator;
pub mod hwconfig;
pub mod resource;
pub mod value;

include!(concat!(env!("OUT_DIR"), "/mikrotik-model.rs"));

#[derive(Deserialize, Debug)]
pub struct Credentials {
    pub user: Box<str>,
    pub password: Box<str>,
}
