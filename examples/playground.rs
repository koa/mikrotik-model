use encoding_rs::mem::decode_latin1;
use mikrotik_model::model::EthernetSpeed;
use mikrotik_model::value::RosValue;

fn main() {
    let value = EthernetSpeed::parse_ros(b"10G-baseCR").ok();
    println!("{:?}, {}", value, decode_latin1( value.unwrap().encode_ros().as_ref()));
}
