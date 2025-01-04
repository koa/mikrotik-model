use config::{Config, Environment, File};
use encoding_rs::mem::decode_latin1;
use env_logger::{Env, TimestampPrecision};
use ipnet::{IpNet, Ipv4Net};
use log::error;
use mikrotik_api::prelude::MikrotikDevice;
use mikrotik_model::model::{IpAddress, IpAddressCfg, IpAddressState};
use mikrotik_model::model::{Resource, ResourceType};
use mikrotik_model::model::{SystemResourceCfg, SystemResourceState, SystemRouterboardSettingsCfg};
use mikrotik_model::resource::{RosResource, SentenceResult};
use mikrotik_model::value::{ParseRosValueResult, RosValue};
use mikrotik_model::Credentials;
use std::any::Any;
use std::net::{IpAddr, Ipv4Addr};
use tokio_stream::StreamExt;

mod test_model {
    use mikrotik_api::error::Error;
    use mikrotik_api::prelude::{ParsedMessage, TrapCategory, TrapResult};
    use mikrotik_model::{resource, value};

    #[derive(Debug, Clone, PartialEq)]
    //#[builder(build_fn(error = "ResourceAccessError"))]
    pub struct SystemResourceCfg {
        pub cpu_frequency: u64,
    }
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct SystemResourceCfgBuilder {
        cpu_frequency: Option<u64>,
    }

    impl resource::DeserializeRosBuilder<SystemResourceCfg> for SystemResourceCfgBuilder {
        type Context = ();
        fn init(ctx: &Self::Context) -> Self {
            Self::default()
        }
        fn append_field(
            &mut self,
            key: &[u8],
            value: Option<&[u8]>,
        ) -> resource::AppendFieldResult {
            match (key, value.as_ref()) {
                (b"cpu-frequency", Some(&value)) => match value::RosValue::parse_ros(value) {
                    value::ParseRosValueResult::None => {
                        resource::AppendFieldResult::InvalidValue(b"cpu-frequency")
                    }
                    value::ParseRosValueResult::Value(v) => {
                        self.cpu_frequency = Some(v);
                        resource::AppendFieldResult::Appended
                    }
                    value::ParseRosValueResult::Invalid => {
                        resource::AppendFieldResult::InvalidValue(b"cpu-frequency")
                    }
                },
                _ => resource::AppendFieldResult::UnknownField,
            }
        }

        fn build(self) -> Result<SystemResourceCfg, &'static [u8]> {
            Ok(SystemResourceCfg {
                cpu_frequency: self.cpu_frequency.ok_or(b"cpu-frequency" as &[u8])?,
            })
        }
    }
    impl resource::DeserializeRosResource for SystemResourceCfg {
        type Builder = SystemResourceCfgBuilder;
    }
    impl resource::RosResource for SystemResourceCfg {
        fn path() -> &'static [u8] {
            b"system/resource"
        }
    }
    impl resource::CfgResource for SystemResourceCfg {
        #[allow(clippy::needless_lifetimes)]
        fn changed_values<'a, 'b>(
            &'a self,
            before: &'b Self,
        ) -> impl Iterator<Item = value::KeyValuePair<'a>> {
            [if self.cpu_frequency == before.cpu_frequency {
                None
            } else {
                Some(value::KeyValuePair {
                    key: b"cpu-frequency",
                    value: value::RosValue::encode_ros(&self.cpu_frequency),
                })
            }]
            .into_iter()
            .flatten()
        }
    }
    impl resource::SingleResource for SystemResourceCfg {}
    impl resource::Updatable for SystemResourceCfg {
        fn calculate_update<'a>(&'a self, from: &'a Self) -> resource::ResourceMutation<'a> {
            resource::ResourceMutation {
                resource: <SystemResourceCfg as resource::RosResource>::path(),
                operation: resource::ResourceMutationOperation::UpdateSingle,
                fields: resource::CfgResource::changed_values(self, from).collect(),
            }
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
    let router = IpAddr::V4(Ipv4Addr::new(10, 192, 5, 7));
    //let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 51));
    println!("{router}");
    let device: MikrotikDevice<SentenceResult<Resource>> = MikrotikDevice::connect(
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
