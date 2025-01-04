use config::{Config, Environment, File};
use env_logger::{Env, TimestampPrecision};
use mikrotik_model::model::IpAddress;
use mikrotik_model::model::ResourceType;
use mikrotik_model::resource::{RosResource, SentenceResult};
use mikrotik_model::{Credentials, MikrotikDevice};
use std::net::{IpAddr, Ipv4Addr};
use tokio_stream::StreamExt;

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
    let router = IpAddr::V4(Ipv4Addr::new(10, 192, 5, 7));
    //let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 51));
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
                for warning in warnings {
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
