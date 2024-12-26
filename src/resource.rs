use crate::model::{InterfaceEthernet, InterfaceEthernetCfg, InterfaceEthernetState};
use crate::value::{KeyValuePair, RosValue};
use crate::{resource, value};
use log::{error, info};
use mikrotik_rs::error::{CommandError, DeviceError};
use mikrotik_rs::protocol::command::{Command, CommandBuilder};
use mikrotik_rs::protocol::{CommandResponse, FatalResponse, TrapResponse};
use mikrotik_rs::MikrotikDevice;
use std::collections::HashMap;
use thiserror::Error;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

#[derive(Debug, Error)]
pub enum ResourceAccessError {
    #[error("Missing field {field_name}")]
    MissingFieldError { field_name: &'static str },
    #[error("Failed to parse field {field_name}: {value}")]
    InvalidValueError {
        field_name: &'static str,
        value: Box<str>,
    },
}

pub trait DeserializeRosResource: Sized {
    fn parse(values: &HashMap<String, Option<String>>) -> Result<Self, ResourceAccessError>;
    fn path() -> &'static str;
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Cannot parse result: {0}")]
    ResourceAccess(#[from] ResourceAccessError),
    #[error("Cannot access device: {0}")]
    Device(#[from] DeviceError),
    #[error("Fatal error from device: {0}")]
    Fatal(FatalResponse),
    #[error("Trap from device: {0}")]
    Trap(TrapResponse),
    #[error("Cannot build query: {0}")]
    InvalidQueryParameter(#[from] CommandError),
}

pub async fn stream_result<R: DeserializeRosResource>(
    cmd: Command,
    device: &MikrotikDevice,
) -> impl Stream<Item = Result<R, Error>> {
    ReceiverStream::new(device.send_command(cmd).await).filter_map(|res| {
        //println!(">> Get System Res Response {:?}", res);
        match res {
            Ok(CommandResponse::Reply(r)) => {
                Some(R::parse(&r.attributes).map_err(Error::ResourceAccess))
            }
            Ok(CommandResponse::Fatal(e)) => Some(Err(Error::Fatal(e))),
            Ok(CommandResponse::Trap(e)) => Some(Err(Error::Trap(e))),
            Ok(CommandResponse::Done(_)) => None,
            Err(e) => {
                error!("Cannot fetch ROS resource: {e}");
                Some(Err(Error::Device(e)))
            }
        }
    })
}

pub async fn stream_resource<R: DeserializeRosResource>(
    device: &MikrotikDevice,
) -> impl Stream<Item = Result<R, Error>> {
    let cmd = CommandBuilder::new()
        .command(&format!("/{}/print", R::path()))
        .expect("Invalid path")
        .build();
    stream_result(cmd, device).await
}

pub async fn list_resources<R: DeserializeRosResource>(
    device: &MikrotikDevice,
) -> impl Stream<Item = R> {
    let cmd = CommandBuilder::new()
        .command(&format!("/{}/print", R::path()))
        .expect("Invalid path")
        .build();
    ReceiverStream::new(device.send_command(cmd).await).filter_map(|res| {
        println!(">> Get System Res Response {:?}", res);
        match res {
            Ok(CommandResponse::Reply(r)) => match R::parse(&r.attributes) {
                Ok(resource) => Some(resource),
                Err(e) => {
                    error!("Cannot parse ROS resource: {e}");
                    None
                }
            },
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

pub trait RosResource: Sized {
    fn known_fields() -> &'static [&'static str];
}

pub trait SingleResource: DeserializeRosResource {
    /*async fn fetch(device: &MikrotikDevice)->Result<Option<Self>, Error>{
        if let Some(row) = stream_resource::<Self>(device).await.next().await{
            Ok(Some(row?))
        }else{
            Ok(None)
        }
    }*/
    fn fetch(
        device: &MikrotikDevice,
    ) -> impl std::future::Future<Output = Result<Option<Self>, Error>> + Send {
        async {
            if let Some(row) = stream_resource::<Self>(device).await.next().await {
                Ok(Some(row?))
            } else {
                Ok(None)
            }
        }
    }
}
pub trait KeyedResource: DeserializeRosResource {
    type Key: RosValue;
    fn key_name() -> &'static str;
    fn key_value(&self) -> &Self::Key;
    fn fetch_all(
        device: &MikrotikDevice,
    ) -> impl std::future::Future<Output = Result<Box<[Self]>, Error>> + Send
    where
        <Self as KeyedResource>::Key: Sync,
        Self: Send,
    {
        async {
            let cmd = CommandBuilder::new()
                .command(&format!("/{}/print", Self::path()))
                .expect("Invalid path")
                .build();
            stream_result::<Self>(cmd, device)
                .await
                .collect::<Result<Box<[_]>, _>>()
                .await
        }
    }
    fn fetch(
        device: &MikrotikDevice,
        key: &Self::Key,
    ) -> impl std::future::Future<Output = Result<Option<Self>, Error>> + Send
    where
        <Self as KeyedResource>::Key: Sync,
    {
        async {
            let cmd = CommandBuilder::new()
                .command(&format!("/{}/print", Self::path()))
                .expect("Invalid path")
                .query_equal(Self::key_name(), key.encode_ros().as_ref())?
                .build();
            if let Some(row) = stream_result(cmd, device).await.next().await {
                Ok(Some(row?))
            } else {
                Ok(None)
            }
        }
    }
}
pub trait CfgResource: DeserializeRosResource {
    #[allow(clippy::needless_lifetimes)]
    fn changed_values<'a, 'b>(&'a self, before: &'b Self)
        -> impl Iterator<Item = KeyValuePair<'a>>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceEthernetById {
    id: Box<str>,
    interface: InterfaceEthernetCfg,
}
impl DeserializeRosResource for InterfaceEthernetById {
    fn parse(values: &HashMap<String, Option<String>>) -> Result<Self, ResourceAccessError> {
        Ok(Self {
            id: values
                .get(".id")
                .and_then(|v| v.as_ref())
                .map(|value| match value::RosValue::parse_ros(value.as_str()) {
                    value::ParseRosValueResult::None => {
                        Err(resource::ResourceAccessError::MissingFieldError { field_name: ".id" })
                    }
                    value::ParseRosValueResult::Value(v) => Ok(v),
                    value::ParseRosValueResult::Invalid => {
                        Err(resource::ResourceAccessError::InvalidValueError {
                            field_name: ".id",
                            value: value.clone().into_boxed_str(),
                        })
                    }
                })
                .unwrap_or(Err(resource::ResourceAccessError::MissingFieldError {
                    field_name: ".id",
                }))?,
            interface: InterfaceEthernetCfg::parse(values)?,
        })
    }

    fn path() -> &'static str {
        InterfaceEthernet::path()
    }
}
impl KeyedResource for InterfaceEthernetById {
    type Key = Box<str>;

    fn key_name() -> &'static str {
        ".id"
    }

    fn key_value(&self) -> &Box<str> {
        &self.id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceMutation<'a> {
    pub resource: &'static str,
    pub operation: ResourceMutationOperation<'a>,
    pub fields: Box<[KeyValuePair<'a>]>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceMutationOperation<'a> {
    Add,
    RemoveByKey(KeyValuePair<'a>),
    UpdateSingle,
    UpdateByKey(KeyValuePair<'a>),
}

pub trait Updatable {
    fn calculate_update<'a>(&'a self, from: &'a Self) -> ResourceMutation<'a>;
}

impl<R: KeyedResource + CfgResource> Updatable for R {
    fn calculate_update<'a>(&'a self, from: &'a Self) -> ResourceMutation<'a> {
        ResourceMutation {
            resource: R::path(),
            operation: ResourceMutationOperation::UpdateByKey(KeyValuePair {
                key: R::key_name(),
                value: from.key_value().encode_ros(),
            }),
            fields: self.changed_values(from).collect(),
        }
    }
}