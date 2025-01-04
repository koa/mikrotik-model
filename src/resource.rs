use crate::value::{KeyValuePair, RosValue};
use encoding_rs::mem::decode_latin1;
use log::error;

use crate::resource;
use mikrotik_api::prelude::{ParsedMessage, TrapCategory, TrapResult};
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResourceAccessError {
    #[error("Missing field {}",decode_latin1(.field_name))]
    MissingFieldError { field_name: &'static [u8] },
    #[error("Failed to parse field {}: {}",decode_latin1(.field_name),decode_latin1(.value))]
    InvalidValueError {
        field_name: &'static [u8],
        value: Box<[u8]>,
    },
}

#[derive(Error)]
pub enum ResourceAccessWarning {
    #[error("Unexpected field received {}",decode_latin1(.field_name))]
    UnexpectedFieldError { field_name: Box<[u8]> },
}
impl Debug for ResourceAccessWarning {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceAccessWarning::UnexpectedFieldError { field_name } => {
                write!(f, "Unexpected field: {}", decode_latin1(field_name),)
            }
        }
    }
}

pub trait DeserializeRosResource: Sized {
    type Builder: DeserializeRosBuilder<Self>;
}
pub trait DeserializeRosBuilder<R: DeserializeRosResource> {
    type Context: Send + Sync + Debug;
    fn init(context: &Self::Context) -> Self;
    fn append_field(&mut self, key: &[u8], value: Option<&[u8]>) -> AppendFieldResult;
    fn build(self) -> Result<R, &'static [u8]>;
}
#[derive(Debug, Copy, Clone)]
pub enum AppendFieldResult {
    Appended,
    InvalidValue(&'static [u8]),
    UnknownField,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Cannot parse result: {0}")]
    ResourceAccess(#[from] ResourceAccessError),
    #[error("Cannot access device: {0}")]
    Device(#[from] mikrotik_api::error::Error),
    //#[error("Fatal error from device: {0}")]
    //Fatal(FatalResponse),
    #[error("Trap from device: {0}")]
    Trap(TrapResponse),
}

#[derive(Debug, Eq, PartialEq, Hash)]
struct TrapResponse {
    pub category: Option<TrapCategory>,
    pub message: Box<[u8]>,
}

impl From<&TrapResult<'_>> for TrapResponse {
    fn from(value: &TrapResult) -> Self {
        Self {
            category: value.category,
            message: Box::from(value.message),
        }
    }
}
impl Display for TrapResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            self.category.map(|c| c.description()).unwrap_or_default(),
            decode_latin1(&self.message)
        )
    }
}

/*pub async fn stream_resource<R: DeserializeRosResource>(
    device: &MikrotikDevice<R>,
) -> impl Stream<Item = Result<R, Error>> {
    let cmd = CommandBuilder::new()
        .command(&[b"/", R::path(), b"/print"])
        .build();
    device.send_simple_command([b"/", R::path(), b"/print"].concat());
    stream_result(cmd, device).await
}

pub async fn list_resources<R: DeserializeRosResource>(
    device: &MikrotikDevice,
) -> impl Stream<Item = R> {
    let cmd = CommandBuilder::new()
        .command(&[b"/", R::path(), b"/print"])
        .build();
    ReceiverStream::new(device.send_command(cmd).await).filter_map(|res| {
        println!(">> Get System Res Response {:?}", res);
        match res {
            Ok(CommandResponse::Reply(r)) => match R::parse(&get_attributes(&r)) {
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
*/
pub trait RosResource: Sized {
    fn path() -> &'static [u8];
}

pub trait SingleResource: DeserializeRosResource {
    /*fn fetch(
        device: &MikrotikDevice,
    ) -> impl std::future::Future<Output = Result<Option<Self>, Error>> + Send {
        async {
            if let Some(row) = stream_resource::<Self>(device).await.next().await {
                Ok(Some(row?))
            } else {
                Ok(None)
            }
        }
    }*/
}
pub trait KeyedResource: DeserializeRosResource {
    type Key: RosValue;
    fn key_name() -> &'static [u8];
    fn key_value(&self) -> &Self::Key;
    /*fn fetch_all(
        device: &MikrotikDevice,
    ) -> impl std::future::Future<Output = Result<Box<[Self]>, Error>> + Send
    where
        <Self as KeyedResource>::Key: Sync,
        Self: Send,
    {
        async {
            let cmd = CommandBuilder::new()
                .command(&[b"/", Self::path(), b"/print"])
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
                .command(&[b"/", Self::path(), b"/print"])
                .query_equal(Self::key_name(), key.encode_ros().as_ref())
                .build();
            if let Some(row) = stream_result(cmd, device).await.next().await {
                Ok(Some(row?))
            } else {
                Ok(None)
            }
        }
    }*/
}
pub trait CfgResource: DeserializeRosResource {
    #[allow(clippy::needless_lifetimes)]
    fn changed_values<'a, 'b>(&'a self, before: &'b Self)
        -> impl Iterator<Item = KeyValuePair<'a>>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceMutation<'a> {
    pub resource: &'static [u8],
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

impl<R: KeyedResource + CfgResource + RosResource> Updatable for R {
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

pub trait Creatable: CfgResource {
    fn calculate_create(&self) -> ResourceMutation<'_>;
}

#[derive(Debug)]
pub enum SentenceResult<R> {
    Row {
        value: R,
        warnings: Box<[resource::ResourceAccessWarning]>,
    },
    Error {
        errors: Box<[resource::ResourceAccessError]>,
        warnings: Box<[resource::ResourceAccessWarning]>,
    },
    Trap {
        category: Option<TrapCategory>,
        message: Box<[u8]>,
    },
}

impl<R: resource::DeserializeRosResource + Send + 'static> ParsedMessage for SentenceResult<R> {
    type Context = <<R as DeserializeRosResource>::Builder as DeserializeRosBuilder<R>>::Context;

    fn parse_message(sentence: &[(&[u8], Option<&[u8]>)], context: &Self::Context) -> Self {
        let mut builder = R::Builder::init(context);
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        for (key, value) in sentence {
            match builder.append_field(key, value.as_deref()) {
                resource::AppendFieldResult::Appended => {}
                resource::AppendFieldResult::InvalidValue(field_name) => {
                    errors.push(resource::ResourceAccessError::InvalidValueError {
                        field_name,
                        value: value.map(|v| Box::from(v)).unwrap_or_default(),
                    })
                }
                resource::AppendFieldResult::UnknownField => {
                    warnings.push(resource::ResourceAccessWarning::UnexpectedFieldError {
                        field_name: Box::from(*key),
                    })
                }
            }
        }
        if errors.is_empty() {
            match builder.build() {
                Ok(result) => SentenceResult::Row {
                    value: result,
                    warnings: warnings.into_boxed_slice(),
                },
                Err(field_name) => {
                    errors.push(resource::ResourceAccessError::MissingFieldError { field_name });
                    SentenceResult::Error {
                        errors: errors.into_boxed_slice(),
                        warnings: warnings.into_boxed_slice(),
                    }
                }
            }
        } else {
            SentenceResult::Error {
                errors: errors.into_boxed_slice(),
                warnings: warnings.into_boxed_slice(),
            }
        }
    }

    fn process_error(error: &mikrotik_api::error::Error, context: &Self::Context) -> Self {
        todo!()
    }

    fn process_trap(result: TrapResult, context: &Self::Context) -> Self {
        SentenceResult::Trap {
            category: result.category,
            message: Box::from(result.message),
        }
    }
}
