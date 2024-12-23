pub mod hwconfig;
pub mod resource;
pub mod value;

include!(concat!(env!("OUT_DIR"), "/mikrotik-model.rs"));
