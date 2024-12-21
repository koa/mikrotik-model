pub mod resource;
pub mod value;
pub mod model {
    include!(concat!(env!("OUT_DIR"), "/mikrotik-model.rs"));
}
