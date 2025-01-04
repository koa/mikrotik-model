use crate::value::{KeyValuePair, RosValue};
use encoding_rs::mem::decode_latin1;
use log::{debug, error};

use crate::model::{Resource, ResourceType};
use crate::MikrotikDevice;
use mikrotik_api::prelude::{ParsedMessage, TrapCategory, TrapResult};
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;
use tokio_stream::{Stream, StreamExt};

#[derive(Debug, Error, Clone)]
pub enum ResourceAccessError {
    #[error("Missing field {}",decode_latin1(.field_name))]
    MissingFieldError { field_name: &'static [u8] },
    #[error("Failed to parse field {}: {}",decode_latin1(.field_name),decode_latin1(.value))]
    InvalidValueError {
        field_name: &'static [u8],
        value: Box<[u8]>,
    },
    #[error("Error fetching data from mikrotik api {0}")]
    ApiError(mikrotik_api::error::Error),
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
    fn unwrap_resource(resource: Resource) -> Option<Self>;
    fn resource_type() -> ResourceType;
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
pub struct TrapResponse {
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

pub async fn stream_resource<R: DeserializeRosResource + RosResource>(
    device: &MikrotikDevice,
) -> impl Stream<Item = SentenceResult<R>> {
    device
        .send_simple_command(&[b"/", R::path(), b"/print"], R::resource_type())
        .await
        .map(|entry| entry.map(|r| R::unwrap_resource(r).expect("Unexpected result type")))
}

pub trait RosResource: Sized {
    fn path() -> &'static [u8];
}

pub trait SingleResource: DeserializeRosResource + RosResource {
    fn fetch(
        device: &MikrotikDevice,
    ) -> impl std::future::Future<Output = Result<Option<Self>, Error>> + Send {
        async {
            match stream_resource::<Self>(device).await.next().await {
                Some(entry) => value_or_error(entry).map(Some),
                None => Ok(None),
            }
        }
    }
}
fn value_or_error<R: DeserializeRosResource>(entry: SentenceResult<R>) -> Result<R, Error> {
    match entry {
        SentenceResult::Row { value, warnings } => {
            if !warnings.is_empty() {
                debug!("Warnings on fetch {:?}: {warnings:#?}", R::resource_type())
            }
            Ok(value)
        }
        SentenceResult::Error { errors, warnings } => {
            if !warnings.is_empty() {
                debug!("Warnings on fetch {:?}: {warnings:#?}", R::resource_type())
            }
            Err(Error::ResourceAccess(
                errors.first().expect("Error without error").clone(),
            ))
        }
        SentenceResult::Trap { category, message } => {
            Err(Error::Trap(TrapResponse { category, message }))
        }
    }
}

pub trait KeyedResource: DeserializeRosResource + RosResource {
    type Key: RosValue;
    fn key_name() -> &'static [u8];
    fn key_value(&self) -> &Self::Key;
    fn fetch_all(
        device: &MikrotikDevice,
    ) -> impl std::future::Future<Output = Result<Box<[Self]>, Error>> + Send
    where
        <Self as KeyedResource>::Key: Sync,
        Self: Send,
    {
        async {
            stream_resource::<Self>(device)
                .await
                .map(|entry| value_or_error(entry))
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
            Ok(
                match device
                    .send_command(
                        &[b"/", Self::path(), b"/print"],
                        |cmd| cmd.query_equal(Self::key_name(), key.encode_ros()),
                        Self::resource_type(),
                    )
                    .await
                    .map(|entry| {
                        entry.map(|r| Self::unwrap_resource(r).expect("Unexpected result type"))
                    })
                    .next()
                    .await
                {
                    None => None,
                    Some(v) => Some(value_or_error(v)?),
                },
            )
        }
    }
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
        warnings: Box<[ResourceAccessWarning]>,
    },
    Error {
        errors: Box<[ResourceAccessError]>,
        warnings: Box<[ResourceAccessWarning]>,
    },
    Trap {
        category: Option<TrapCategory>,
        message: Box<[u8]>,
    },
}

impl<R> SentenceResult<R> {
    pub fn map<V, F: FnOnce(R) -> V>(self, apply: F) -> SentenceResult<V> {
        match self {
            SentenceResult::Row { value, warnings } => SentenceResult::Row {
                value: apply(value),
                warnings,
            },
            SentenceResult::Error { errors, warnings } => {
                SentenceResult::Error { errors, warnings }
            }
            SentenceResult::Trap { category, message } => {
                SentenceResult::Trap { category, message }
            }
        }
    }
}

impl ParsedMessage for SentenceResult<Resource> {
    type Context = ResourceType;

    fn parse_message(sentence: &[(&[u8], Option<&[u8]>)], context: &Self::Context) -> Self {
        let mut builder = context.create_builder();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        for (key, value) in sentence {
            match builder.append_field(key, value.as_deref()) {
                AppendFieldResult::Appended => {}
                AppendFieldResult::InvalidValue(field_name) => {
                    errors.push(ResourceAccessError::InvalidValueError {
                        field_name,
                        value: value.map(Box::from).unwrap_or_default(),
                    })
                }
                AppendFieldResult::UnknownField => {
                    warnings.push(ResourceAccessWarning::UnexpectedFieldError {
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
                    errors.push(ResourceAccessError::MissingFieldError { field_name });
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

    fn process_error(error: &mikrotik_api::error::Error, _context: &Self::Context) -> Self {
        SentenceResult::Error {
            errors: Box::new([ResourceAccessError::ApiError(error.clone())]),
            warnings: Box::new([]),
        }
    }

    fn process_trap(result: TrapResult, _context: &Self::Context) -> Self {
        SentenceResult::Trap {
            category: result.category,
            message: Box::from(result.message),
        }
    }
}

impl<R: DeserializeRosResource + Send + 'static> ParsedMessage for SentenceResult<R> {
    type Context = <<R as DeserializeRosResource>::Builder as DeserializeRosBuilder<R>>::Context;

    fn parse_message(sentence: &[(&[u8], Option<&[u8]>)], context: &Self::Context) -> Self {
        let mut builder = R::Builder::init(context);
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        for (key, value) in sentence {
            match builder.append_field(key, value.as_deref()) {
                AppendFieldResult::Appended => {}
                AppendFieldResult::InvalidValue(field_name) => {
                    errors.push(ResourceAccessError::InvalidValueError {
                        field_name,
                        value: value.map(Box::from).unwrap_or_default(),
                    })
                }
                AppendFieldResult::UnknownField => {
                    warnings.push(ResourceAccessWarning::UnexpectedFieldError {
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
                    errors.push(ResourceAccessError::MissingFieldError { field_name });
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

    fn process_error(error: &mikrotik_api::error::Error, _context: &Self::Context) -> Self {
        SentenceResult::Error {
            errors: Box::new([ResourceAccessError::ApiError(error.clone())]),
            warnings: Box::new([]),
        }
    }

    fn process_trap(result: TrapResult, _context: &Self::Context) -> Self {
        SentenceResult::Trap {
            category: result.category,
            message: Box::from(result.message),
        }
    }
}
