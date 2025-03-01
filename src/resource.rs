use crate::ascii::AsciiString;
use crate::generator::Generator;
use crate::resource::Error::ResourceAccess;
use crate::{
    model::{ReferenceType, Resource, ResourceType},
    value::{KeyValuePair, RosValue},
    MikrotikDevice,
};
use encoding_rs::mem::decode_latin1;
use log::{debug, error, info};
use mikrotik_api::prelude::{CommandBuilder, ParsedMessage, TrapCategory, TrapResult};
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Write};
use std::hash::Hash;
use thiserror::Error;
use tokio_stream::{FromStream, Stream, StreamExt};

#[derive(Debug, Error, Clone)]
pub enum ResourceAccessError {
    #[error("Missing field {}",decode_latin1(.field_name))]
    MissingFieldError { field_name: &'static [u8] },
    #[error("Undefined field {}",decode_latin1(.field_name))]
    UndefinedFieldError { field_name: &'static [u8] },
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

pub trait FieldUpdateHandler {
    #[inline]
    fn update_reference<V: RosValue + 'static>(
        &mut self,
        _ref_type: ReferenceType,
        _old_value: &V,
        _new_value: &V,
    ) -> bool {
        false
    }
}

pub trait DeserializeRosResource: Sized + FieldUpdateHandler {
    type Builder: DeserializeRosBuilder<Self>;
    fn unwrap_resource(resource: Resource) -> Option<Self>;
    fn resource_type() -> ResourceType;
    #[inline]
    fn generate_derived_updates<V: FieldUpdateHandler>(
        &self,
        before_value: &Self,
        handler: &mut V,
    ) {
    }
    /*fn update<R, T: UpdateHandler<R>>(&self, _handler: T) -> Option<R> {
        None
    }
    fn create<R, T: CreateHandler<R>>(&self, _handler: T) -> Option<R> {
        None
    }*/
}
pub trait UpdateHandler<R> {
    fn handle_updatable<T: Updatable>(self, value: &T) -> R;
}
pub trait CreateHandler<R> {
    fn handle_creatable<T: Creatable>(self, value: &T) -> R;
}

pub trait CompositeRosResource: Sized + DeserializeRosResource {
    type ReadOnlyData: DeserializeRosResource;
    type ReadWriteData: DeserializeRosResource + Updatable;
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
    #[error("Cannot parse result: {resource_type:?} {error}")]
    ResourceAccess {
        error: ResourceAccessError,
        resource_type: ResourceType,
    },
    #[error("Cannot access device: {0}")]
    Device(#[from] mikrotik_api::error::Error),
    //#[error("Fatal error from device: {0}")]
    //Fatal(FatalResponse),
    #[error("Trap from device: {0}")]
    Trap(TrapResponse),
    #[error("Cannot fetch single item")]
    ErrorFetchingSingleItem,
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
    //println!("Fetch: {}", decode_latin1(R::path()));
    device
        .send_simple_command(&[b"/", R::path(), b"/print"], R::resource_type())
        .await
        .map(|entry| entry.map(|r| R::unwrap_resource(r).expect("Unexpected result type")))
}

pub trait RosResource: Sized {
    fn path() -> &'static [u8];
    fn provides_reference(&self) -> impl Iterator<Item = (ReferenceType, Cow<[u8]>)>;
    fn consumes_reference(&self) -> impl Iterator<Item = (ReferenceType, Cow<[u8]>)>;
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
            if !errors.is_empty() {
                info!("Errors from fetch {:?}", R::resource_type());
                for error in &errors {
                    info!("- {}", error);
                }
            }
            Err(Error::ResourceAccess {
                error: errors.first().expect("Error without error").clone(),
                resource_type: R::resource_type(),
            })
        }
        SentenceResult::Trap { category, message } => {
            Err(Error::Trap(TrapResponse { category, message }))
        }
    }
}

pub trait KeyedResource: DeserializeRosResource + RosResource {
    type Key: RosValue;
    type Value: DeserializeRosResource + RosResource;
    fn key_name() -> &'static [u8];
    fn key_value(&self) -> &Self::Key;
    fn value(&self) -> &Self::Value;
    fn filter(cmd: CommandBuilder) -> CommandBuilder {
        cmd
    }
    fn fetch_all<T: FromStream<Self> + Send>(
        device: &MikrotikDevice,
    ) -> impl std::future::Future<Output = Result<T, Error>>
    where
        <Self as KeyedResource>::Key: Sync,
        Self: Send,
    {
        async {
            device
                .send_command(
                    &[b"/", Self::path(), b"/print"],
                    |cmd| Self::filter(cmd.query_is_present(Self::key_name())),
                    Self::resource_type(),
                )
                .await
                .map(|entry| {
                    entry.map(|r| Self::unwrap_resource(r).expect("Unexpected result type"))
                })
                .map(|entry| value_or_error(entry))
                .collect::<Result<T, _>>()
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
                        |cmd| Self::filter(cmd.query_equal(Self::key_name(), key.encode_ros())),
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
pub trait SetResource<Base: RosResource>: FieldUpdateHandler {
    #[allow(clippy::needless_lifetimes)]
    fn changed_values<'a, 'b>(&'a self, before: &'b Base)
        -> impl Iterator<Item = KeyValuePair<'a>>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceMutation<'a> {
    pub resource: &'static [u8],
    pub operation: ResourceMutationOperation<'a>,
    pub fields: Box<[KeyValuePair<'a>]>,
    pub depends: Box<[(ReferenceType, Cow<'a, [u8]>)]>,
    pub provides: Box<[(ReferenceType, Cow<'a, [u8]>)]>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceMutationOperation<'a> {
    Add,
    RemoveByKey(KeyValuePair<'a>),
    UpdateSingle,
    UpdateByKey(KeyValuePair<'a>),
}

pub trait Updatable: DeserializeRosResource {
    type From: RosResource;
    fn calculate_update<'a>(&'a self, from: &'a Self::From) -> ResourceMutation<'a>;
    fn update<R, T: UpdateHandler<R>>(&self, handler: T) -> Option<R> {
        Some(handler.handle_updatable(self))
    }
}

/*impl<R: KeyedResource + CfgResource + RosResource> Updatable for R {
    type From = R;
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
}*/
/*impl<R: SetResource + KeyedResource> Updatable for R {
    type From = R::Cfg;

    fn calculate_update<'a>(&'a self, from: &'a R::Cfg) -> ResourceMutation<'a> {
        ResourceMutation {
            resource: R::Cfg::path(),
            operation: ResourceMutationOperation::UpdateByKey(KeyValuePair {
                key: R::key_name(),
                value: self.key_value().encode_ros(),
            }),
            fields: self.changed_values(from).collect(),
        }
    }
}*/

pub trait Creatable: CfgResource {
    fn calculate_create(&self) -> ResourceMutation<'_>;
    fn create<R, T: CreateHandler<R>>(&self, handler: T) -> Option<R> {
        Some(handler.handle_creatable(self))
    }
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
                    errors.push(ResourceAccessError::UndefinedFieldError { field_name });
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

#[derive(Debug)]
pub struct UpdatePairing<'a, 'b, Current: KeyedResource, Target: KeyedResource> {
    pub orphaned_entries: Box<[&'a mut Current]>,
    pub matched_entries: Box<[(&'a mut Current, &'b Target)]>,
    pub new_entries: Box<[&'b Target]>,
}

impl<'b, Target, Current> UpdatePairing<'b, 'b, Target, Current>
where
    Target: KeyedResource,
    Current: KeyedResource + Updatable<From = Target>,
    Current::Value: 'static + Sized,
{
    pub fn generate_updates(&self) -> impl Iterator<Item = ResourceMutation> {
        self.orphaned_entries
            .iter()
            .map(|entry| {
                let value = entry.key_value().encode_ros();
                let key = Current::key_name();
                ResourceMutation {
                    resource: Current::path(),
                    operation: ResourceMutationOperation::RemoveByKey(KeyValuePair { key, value }),
                    fields: Box::new([]),
                    depends: Default::default(),
                    provides: Default::default(),
                }
            })
            .chain(
                self.matched_entries
                    .iter()
                    .map(|(original, target)| target.calculate_update(original)),
            ) /*.chain(self.new_entries.iter().map(|entry|{
                  ResourceMutation{
                      resource: Current::path(),
                      operation: ResourceMutationOperation::Add,
                      fields: Box::new([]),
                      depends: entry.consumes_reference().filter(|(_,v)|!v.is_empty()).collect(),
                      provides: entry.provides_reference().filter(|(_,v)|!v.is_empty()).collect(),
                  }
              }))*/
    }
}
impl<'a, 'b, Target: KeyedResource, Current: KeyedResource + Updatable<From = Target>>
    UpdatePairing<'a, 'b, Current, Target>
where
    <Target as KeyedResource>::Key: PartialEq<<Current as KeyedResource>::Key>,
{
    pub fn match_updates_by_key(current: &'a mut [Current], target: &'b [Target]) -> Self {
        let mut orphans = Vec::with_capacity(current.len());
        let mut matched = Vec::with_capacity(current.len().max(target.len()));
        let mut target_refs = target.iter().collect::<Vec<_>>();
        for c in current {
            let key = c.key_value();
            if let Some((found_idx, _)) = target_refs
                .iter()
                .enumerate()
                .find(|(_, t)| t.key_value() == key)
            {
                let t = target_refs.remove(found_idx);
                matched.push((c, t));
            } else {
                orphans.push(c);
            }
        }
        UpdatePairing {
            orphaned_entries: orphans.into_boxed_slice(),
            matched_entries: matched.into_boxed_slice(),
            new_entries: target_refs.into_boxed_slice(),
        }
    }
}
