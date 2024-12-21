use config::{Config, Environment, File};
use log::{error, info};
use mikrotik_model::model::{SystemArchitecture, SystemIdentity};
use mikrotik_model::model::SystemResource;
use mikrotik_model::resource::{ResourceAccessError, RosResource};
use mikrotik_model::value::{ParseRosValueResult, RosValue};
use mikrotik_rs::command::response::CommandResponse;
use mikrotik_rs::command::CommandBuilder;
use mikrotik_rs::MikrotikDevice;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::IpAddr;
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
#[derive(Deserialize, Debug)]
pub struct Credentials {
    pub user: Box<str>,
    pub password: Box<str>,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        let device = MikrotikDevice::connect(
            (router, 8728),
            credentials.user.as_ref(),
            Some(credentials.password.as_ref()),
        )
            .await?;
        let mut stream = get_resource::<SystemResource>(&device).await;
        while let Some(r)= stream.next().await {
            println!("Res: \n{r:#?}");
        }
        let mut stream = get_resource::<SystemIdentity>(&device).await;
        while let Some(r)= stream.next().await {
            println!("Id: \n{r:#?}");
        }
    }
    Ok(())
}
async fn get_resource<R: RosResource>(device: &MikrotikDevice) -> impl Stream<Item=R> {
    let cmd = CommandBuilder::new()
        .command(&format!("/{}/print", R::path()))
        .build();
    ReceiverStream::new(device.send_command(cmd).await).filter_map(|res| {
        println!(">> Get System Res Response {:?}", res);
        match res {
            Ok(CommandResponse::Reply(r)) => {
                match R::parse(&r.attributes) {
                    Ok(resource) => { Some(resource) }
                    Err(e) => {
                        error!("Cannot parse ROS resource: {e}");
                        None
                    }
                }
            }
            Ok(reply) =>  {
                info!("response: {reply:?}");
                None
            },
            Err(e) => {
                error!("Cannot fetch ROS resource: {e}");
                None
            }
        }
    })
}