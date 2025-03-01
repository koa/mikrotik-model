/*use crate::{
resource, value::{self, IpOrInterface},
ascii,
};
use std::{time::Duration, net::IpAddr};
use mac_address::MacAddress;
use ipnet::IpNet;*/
use value::Id;

include!(concat!(env!("OUT_DIR"), "/mikrotik-model.rs"));

mod defaults;
