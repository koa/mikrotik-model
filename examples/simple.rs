use config::{Config, Environment, File};
use env_logger::Env;
use env_logger::TimestampPrecision;
use log::{error, info};
use mikrotik_model::error::Error;
use mikrotik_model::model::InterfaceBridgeVlan;
use mikrotik_model::model::InterfaceVlanCfg;
use mikrotik_model::model::SystemIdentityCfg;
use mikrotik_model::model::SystemRouterboardSettingsCfg;
use mikrotik_model::model::SystemRouterboardState;
use mikrotik_model::model::{InterfaceBridge, IpAddress, IpDhcpClient, IpDhcpClientCfg, IpRoute};
use mikrotik_model::model::{InterfaceBridgePort, InterfaceEthernetCfg};
use mikrotik_model::model::{InterfaceEthernet, SystemResource};
use mikrotik_model::resource::SingleResource;
use mikrotik_model::resource::{stream_result, DeserializeRosResource, RosResource};
use mikrotik_model::Credentials;
use mikrotik_rs::error::DeviceError;
use mikrotik_rs::protocol::command::{Command, CommandBuilder};
use mikrotik_rs::protocol::CommandResponse;
use mikrotik_rs::MikrotikDevice;
use serde::Deserialize;
use std::net::IpAddr;
use tokio_stream::adapters::FilterMap;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
/*
#[derive(Debug)]
struct SystemResource {
    pub architecture_name: SystemArchitecture,
    pub board_name: Box<str>,
    pub cpu: Box<str>,
    pub cpu_frequency: u64,
    pub factory_software: Box<str>,
}

impl RosResource for SystemResource {
    fn parse(values: &HashMap<String, Option<String>>) -> Result<Self, ResourceAccessError> {
        Ok(SystemResource {
            architecture_name: values
                .get("architecture-name")
                .and_then(|v| v.as_ref())
                .map(
                    |value| match SystemArchitecture::parse_ros(value.as_str()) {
                        ParseRosValueResult::None => Err(ResourceAccessError::MissingFieldError {
                            field_name: "architecture-name",
                        }),
                        ParseRosValueResult::Value(v) => Ok(v),
                        ParseRosValueResult::Invalid => {
                            Err(ResourceAccessError::InvalidValueError {
                                field_name: "architecture-name",
                                value: value.clone().into_boxed_str(),
                            })
                        }
                    },
                )
                .unwrap_or(Err(ResourceAccessError::MissingFieldError {
                    field_name: "architecture-name",
                }))?,
            board_name: values
                .get("board-name")
                .and_then(|v| v.as_ref())
                .map(|v| v.clone().into_boxed_str())
                .ok_or(ResourceAccessError::MissingFieldError {
                    field_name: "board-name",
                })?,
            cpu: values
                .get("cpu")
                .and_then(|v| v.as_ref())
                .map(|value| match Box::parse_ros(value.as_str()) {
                    ParseRosValueResult::None => {
                        Err(ResourceAccessError::MissingFieldError { field_name: "cpu" })
                    }
                    ParseRosValueResult::Value(v) => Ok(v),
                    ParseRosValueResult::Invalid => Err(ResourceAccessError::InvalidValueError {
                        field_name: "cpu",
                        value: value.clone().into_boxed_str(),
                    }),
                })
                .unwrap_or(Err(ResourceAccessError::MissingFieldError {
                    field_name: "cpu",
                }))?,
            cpu_frequency: values
                .get("cpu-frequency")
                .and_then(|v| v.as_ref())
                .map(|value| match u64::parse_ros(value.as_str()) {
                    ParseRosValueResult::None => Err(ResourceAccessError::MissingFieldError {
                        field_name: "cpu-frequency",
                    }),
                    ParseRosValueResult::Value(v) => Ok(v),
                    ParseRosValueResult::Invalid => Err(ResourceAccessError::InvalidValueError {
                        field_name: "cpu-frequency",
                        value: value.clone().into_boxed_str(),
                    }),
                })
                .unwrap_or(Err(ResourceAccessError::MissingFieldError {
                    field_name: "cpu-frequency",
                }))?,
            factory_software: values
                .get("factory-software")
                .and_then(|v| v.as_ref())
                .map(|value| match Box::parse_ros(value.as_str()) {
                    ParseRosValueResult::None => {
                        Err(ResourceAccessError::MissingFieldError { field_name: "factory-software" })
                    }
                    ParseRosValueResult::Value(v) => Ok(v),
                    ParseRosValueResult::Invalid => Err(ResourceAccessError::InvalidValueError {
                        field_name: "factory-software",
                        value: value.clone().into_boxed_str(),
                    }),
                })
                .unwrap_or(Err(ResourceAccessError::MissingFieldError {
                    field_name: "factory-software",
                }))?,
        })
    }
}
*/

#[tokio::main]
async fn main() -> Result<(), Error> {
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
    let routers: Box<[IpAddr]> = cfg.get("routers")?;
    for router in routers {
        println!("{router}");
        let device = MikrotikDevice::connect(
            (router, 8728),
            credentials.user.as_ref(),
            Some(credentials.password.as_ref()),
        )
        .await?;
        let system_cfg = SystemIdentityCfg::fetch(&device).await?;
        println!("System cfg: {system_cfg:?}");
        let mut stream = get_resource::<SystemRouterboardSettingsCfg>(&device).await;
        while let Some(r) = stream.next().await {
            println!("Board Settings: \n{r:#?}");
        }
        let mut stream = get_resource::<SystemResource>(&device).await;
        while let Some(r) = stream.next().await {
            //println!("Res: \n{r:#?}");
        }
        let mut stream = get_resource::<SystemIdentityCfg>(&device).await;
        while let Some(r) = stream.next().await {
            //println!("Id: \n{r:#?}");
        }
        let mut stream = get_resource::<InterfaceEthernet>(&device).await;
        while let Some(r) = stream.next().await {
            //println!("Eth: \n{r:#?}");
        }
        let mut stream = get_resource::<InterfaceVlanCfg>(&device).await;
        while let Some(r) = stream.next().await {
            //println!("Vlan: \n{r:#?}");
        }
        let mut stream = get_resource::<InterfaceBridge>(&device).await;
        while let Some(r) = stream.next().await {
            //println!("Vlan: \n{r:#?}");
        }
        let mut stream = get_resource::<InterfaceBridgePort>(&device).await;
        while let Some(r) = stream.next().await {
            //println!("Vlan: \n{r:#?}");
        }
        let mut stream = get_resource::<InterfaceBridgeVlan>(&device).await;
        while let Some(r) = stream.next().await {
            //println!("Vlan: \n{r:#?}");
        }
        let mut stream = get_resource::<IpAddress>(&device).await;
        while let Some(r) = stream.next().await {
            //println!("Vlan: \n{r:#?}");
        }
        let mut stream = get_resource::<IpDhcpClient>(&device).await;
        while let Some(r) = stream.next().await {
            println!("DHCP Client: \n{r:#?}");
        }
        let mut stream = get_resource::<IpRoute>(&device).await;
        while let Some(r) = stream.next().await {
            //println!("Vlan: \n{r:#?}");
        }
        println!("--------------------------------------------");
        let cmd = CommandBuilder::new()
            .command(&format!("/{}/print", InterfaceEthernet::path()))
            .unwrap()
            .query_equal("default-name", "ether1")
            .unwrap()
            .build();
        let rows = stream_result::<InterfaceEthernetCfg>(cmd, &device)
            .await
            .collect::<Result<Box<[_]>, _>>()
            .await?;
        for row in rows {
            println!("e1: {row:?}");
        }
        println!("----------------------------------------------");
    }
    Ok(())
}
async fn get_resource<R: RosResource + DeserializeRosResource>(
    device: &MikrotikDevice,
) -> impl Stream<Item = R> {
    println!("{}", R::path());
    let cmd = CommandBuilder::new()
        .command(&format!("/{}/print", R::path()))
        .unwrap()
        .build();
    fetch_results(device, cmd).await
}

async fn fetch_results<R: RosResource + DeserializeRosResource>(
    device: &MikrotikDevice,
    cmd: Command,
) -> impl Stream<Item = R> {
    ReceiverStream::new(device.send_command(cmd).await).filter_map(|res| {
        //println!(">> Get System Res Response {:?}", res);
        match res {
            Ok(CommandResponse::Reply(r)) => {
                for (field_name, value) in r
                    .attributes
                    .iter()
                    .filter(|(name, _)| !R::known_fields().contains(&name.as_str()))
                {
                    error!("new field found: {field_name}: {value:?}",);
                }
                match R::parse(&r.attributes) {
                    Ok(resource) => Some(resource),
                    Err(e) => {
                        error!("Cannot parse ROS resource: {e}");
                        None
                    }
                }
            }
            Ok(CommandResponse::Done(_)) => None,
            Ok(reply) => {
                info!("response: {reply:?}");
                None
            }
            Err(e) => {
                error!("Cannot fetch ROS resource: {e}");
                None
            }
        }
    })
}
