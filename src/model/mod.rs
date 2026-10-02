include!(concat!(env!("OUT_DIR"), "/mikrotik-model.rs"));

mod defaults;
mod enums;
#[cfg(test)]
mod bgp_tests;
