use config::{Config, Environment, File};
use env_logger::{Env, TimestampPrecision};
use mikrotik_model::ascii::AsciiString;
use mikrotik_model::model::{InterfaceEthernetCfg, IpAddress};
use mikrotik_model::model::{ReferenceType, ResourceType};
use mikrotik_model::resource::{FieldUpdateHandler, RosResource, SentenceResult, SetResource};
use mikrotik_model::value::{KeyValuePair, RosValue};
use mikrotik_model::{Credentials, MikrotikDevice, ascii};
use std::borrow::Cow;
use std::net::{IpAddr, Ipv4Addr};
use tokio_stream::StreamExt;

struct InterfaceEthernetSet {
    pub name: AsciiString,
    pub default_name: AsciiString,
}

impl FieldUpdateHandler for InterfaceEthernetSet {
    fn update_reference<V: RosValue + 'static>(
        &mut self,
        ref_type: ReferenceType,
        old_value: &V,
        new_value: &V,
    ) -> bool {
        let old_value_any = old_value as &dyn core::any::Any;
        let new_value_any = new_value as &dyn core::any::Any;

        if let ReferenceType::Interface = ref_type {
            if let (Some(old_value), Some(new_value)) = (
                old_value_any.downcast_ref::<ascii::AsciiString>(),
                new_value_any.downcast_ref::<ascii::AsciiString>(),
            ) {
                let mut modified = false;
                if old_value == &self.name {
                    self.name = new_value.clone();
                    modified = true;
                }
                modified
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .parse_env(Env::default().filter_or("LOG_LEVEL", "info"))
        .format_timestamp(Some(TimestampPrecision::Millis))
        .init();

    let cfg = Config::builder()
        .add_source(File::with_name("routers.yaml"))
        .add_source(
            Environment::with_prefix("APP")
                .separator("-")
                .prefix_separator("_"),
        )
        .build()?;
    let credentials: Credentials = cfg.get("credentials")?;
    //let router = IpAddr::V4(Ipv4Addr::new(10, 192, 5, 7));
    //let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 51));
    let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 1));
    println!("{router}");
    let device: MikrotikDevice = MikrotikDevice::connect(
        (router, 8728),
        credentials.user.as_bytes(),
        Some(credentials.password.as_bytes()),
    )
    .await?;
    let mut stream = device
        .send_simple_command(
            &[b"/", IpAddress::path(), b"/print"],
            ResourceType::IpAddress,
        )
        .await;
    while let Some(result) = stream.next().await {
        match result {
            SentenceResult::Row { value, warnings } => {
                println!("{value:?}");
                for warning in warnings.iter() {
                    println!("warning: {}", warning);
                }
            }
            SentenceResult::Error { errors, warnings } => {
                errors.iter().for_each(|e| println!("error: {}", e));
                warnings
                    .iter()
                    .for_each(|warning| println!("warning: {}", warning));
            }
            SentenceResult::Trap { category, message } => {}
        }
    }

    /* let value = EthernetSpeed::parse_ros(b"10G-baseCR").ok();
    println!(
        "{:?}, {}",
        value,
        decode_latin1(value.unwrap().encode_ros().as_ref())
    );

    let data: Vec<(&[u8], Option<&[u8]>)> = vec![
        (b"address", Some(b"127.0.0.1/8")),
        (b"interface", Some(b"lo")),
        (b"Hello", Some(b"World!")),
    ];
    let mut cfg = IpAddressCfgBuilder::default();
    for (key, value) in data {
        if let Some(value) = value {
            match key {
                b"address" => {
                    match RosValue::parse_ros(value) {
                        ParseRosValueResult::Value(v) => {
                            cfg.address(v);
                        }
                        ParseRosValueResult::None => {}
                        ParseRosValueResult::Invalid => {
                            error!("Cannot parse address");
                        }
                    };
                }
                b"interface" => {
                    match RosValue::parse_ros(value) {
                        ParseRosValueResult::Value(v) => {
                            cfg.interface(v);
                        }
                        ParseRosValueResult::None => {}
                        ParseRosValueResult::Invalid => {
                            error!("Cannot parse interface");
                        }
                    };
                }
                b"comment" => {
                    match <Option<Box<[u8]>> as RosValue>::parse_ros(value) {
                        ParseRosValueResult::Value(v) => {
                            cfg.comment(v);
                        }
                        ParseRosValueResult::None => {}
                        ParseRosValueResult::Invalid => {
                            error!("Cannot parse comment");
                        }
                    };
                }
                field => {
                    error!("Unsupported field {:?}", decode_latin1(field));
                }
            }
        }
    }

    println!("{:?}", cfg.build());*/
    Ok(())
}
