use crate::value::Id;
use crate::{
    MikrotikDevice,
    ascii::AsciiString,
    hwconfig::DeviceType,
    model::{ReferenceType, Resource, ResourceRef, ResourceType},
    value::{KeyValuePair, RosValue},
};
use encoding_rs::mem::decode_latin1;
use itertools::Itertools;
use log::{debug, info};
use mikrotik_api::prelude::{CommandBuilder, ParsedMessage, TrapCategory, TrapResult};
use mikrotik_api::simple::SimpleResult;
use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::{Debug, Display, Formatter},
    hash::Hash,
    sync::Arc,
};
use thiserror::Error;
use tokio_stream::{FromStream, Stream, StreamExt, empty};
use tokio_util::either::Either;

#[derive(Debug, Error, Clone)]
pub enum ResourceAccessError {
    #[error("Missing field {}", decode_latin1(.field_name))]
    MissingFieldError { field_name: &'static [u8] },
    #[error("Undefined field {}", decode_latin1(.field_name))]
    UndefinedFieldError { field_name: &'static [u8] },
    #[error("Failed to parse field {}: {}", decode_latin1(.field_name), decode_latin1(.value))]
    InvalidValueError {
        field_name: &'static [u8],
        value: Box<[u8]>,
    },
    #[error("Error fetching data from mikrotik api {0}")]
    ApiError(mikrotik_api::error::Error),
}

#[derive(Error)]
pub enum ResourceAccessWarning {
    #[error("Unexpected field received {}", decode_latin1(.field_name))]
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
        _before_value: &Self,
        _handler: &mut V,
    ) {
    }
}
pub trait UpdateHandler<R> {
    fn handle_updatable<F: RosResource, T: Updatable<F>>(self, value: &T) -> R;
}
pub trait CreateHandler<R> {
    fn handle_creatable<T: Creatable>(self, value: &T) -> R;
}

pub trait CompositeRosResource: Sized + DeserializeRosResource {
    type ReadOnlyData: DeserializeRosResource;
    type ReadWriteData: DeserializeRosResource + Updatable<Self::ReadWriteData> + RosResource;
}

pub trait DeserializeRosBuilder<R: DeserializeRosResource> {
    type Context: Send + Sync + Debug + Clone;
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
    #[error("Cannot identify type {0}")]
    UnknownType(AsciiString),
    #[error("No {} entry found matching {}={}", decode_latin1(.resource), decode_latin1(.key), decode_latin1(.value))]
    KeyNotFound {
        resource: &'static [u8],
        key: &'static [u8],
        value: Box<[u8]>,
    },
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
pub async fn monitor_once_resource<R: DeserializeRosResource + RosResource>(
    entries: impl Iterator<Item = Id>,
    device: &MikrotikDevice,
) -> impl Stream<Item = SentenceResult<R>> {
    let mut id_list = Vec::<u8>::new();
    for id in entries {
        if !id_list.is_empty() {
            id_list.extend(b",");
        }
        id_list.extend(id.encode_ros().as_ref());
    }

    if id_list.is_empty() {
        Either::Left(empty())
    } else {
        Either::Right(
            device
                .send_command(
                    &[b"/", R::path(), b"/monitor"],
                    |cb| {
                        cb.attribute(b".id", id_list.as_slice())
                            .attribute(b"once", b"true")
                    },
                    R::resource_type(),
                )
                .await
                .map(|entry| entry.map(|r| R::unwrap_resource(r).expect("Unexpected result type"))),
        )
    }
}

pub async fn collect_resource<R: DeserializeRosResource + RosResource>(
    device: &MikrotikDevice,
) -> Result<Box<[R]>, Error> {
    stream_resource(device)
        .await
        .map(|entry| value_or_error(entry))
        .collect::<Result<Box<[R]>, _>>()
        .await
}

pub trait RosResource: Sized {
    fn path() -> &'static [u8];
    fn provides_reference(&'_ self) -> impl Iterator<Item = (ReferenceType, Cow<'_, [u8]>)>;
    fn consumes_reference(&'_ self) -> impl Iterator<Item = (ReferenceType, Cow<'_, [u8]>)>;
    /// The values of its `unique` fields (field name, value): no two entries of the menu may
    /// hold the same one (`vni` of `/interface/vxlan`).
    fn unique_values(&'_ self) -> impl Iterator<Item = (&'static [u8], Cow<'_, [u8]>)> {
        std::iter::empty()
    }
    /// Its references marked `blocks-removal`, which have to be gone before what they name is
    /// removed: RouterOS refuses the removal (a radio's `datapath`) or takes this entry with it
    /// (a vtep's `interface`). Other references don't hold the entry back: removing it leaves
    /// them dangling until they are set anew.
    fn removal_blocking_references(
        &'_ self,
    ) -> impl Iterator<Item = (ReferenceType, Cow<'_, [u8]>)> {
        std::iter::empty()
    }
    fn create_resource_ref(&'_ self) -> ResourceRef<'_>;
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
                for error in errors.iter() {
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
    /// What the key names when it refers to another entry (`ipv6/nd`'s `interface`): a
    /// `[find interface=X]` only finds the entry while X is there under that name.
    fn key_reference() -> Option<ReferenceType> {
        None
    }
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

type Reference<'a> = (ReferenceType, Cow<'a, [u8]>);

/// A value of a `unique` field: the menu (`path`), the field and the value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UniqueValue<'a> {
    pub resource: &'static [u8],
    pub field: &'static [u8],
    pub value: Cow<'a, [u8]>,
}

impl UniqueValue<'_> {
    fn into_owned(self) -> UniqueValue<'static> {
        UniqueValue {
            resource: self.resource,
            field: self.field,
            value: Cow::Owned(self.value.into_owned()),
        }
    }
}

/// What a mutation takes from and gives back to the others besides the names it needs
/// (`depends`) and makes (`provides`); `sort_mutations` derives the order from it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MutationOrdering<'a> {
    /// names it takes away: a removal those of the entry, a rename the old one
    pub releases: Box<[Reference<'a>]>,
    /// `unique` values it gives an entry
    pub claims: Box<[UniqueValue<'a>]>,
    /// `unique` values it takes from an entry: a removal, or an update to another value
    pub frees: Box<[UniqueValue<'a>]>,
    /// `blocks-removal` references the entry had before and no more: the entry they name is
    /// removed only after (RouterOS refuses to remove a datapath a radio uses, and removes the
    /// vteps with their VXLAN)
    pub drops: Box<[Reference<'a>]>,
    /// the names its `[find ...]` looks for (`KeyedResource::key_reference`), which must not
    /// be removed or renamed before
    pub finds: Box<[Reference<'a>]>,
}

impl MutationOrdering<'_> {
    pub fn into_owned(self) -> MutationOrdering<'static> {
        fn owned(references: Box<[Reference<'_>]>) -> Box<[Reference<'static>]> {
            references
                .into_iter()
                .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
                .collect()
        }
        MutationOrdering {
            releases: owned(self.releases),
            claims: self.claims.into_iter().map(UniqueValue::into_owned).collect(),
            frees: self.frees.into_iter().map(UniqueValue::into_owned).collect(),
            drops: owned(self.drops),
            finds: owned(self.finds),
        }
    }

    /// Adding `target`: it claims its unique values.
    pub fn create<'a, T: RosResource>(target: &'a T) -> MutationOrdering<'a> {
        MutationOrdering {
            claims: unique_values(target).collect(),
            ..Default::default()
        }
    }

    /// Updating `from` into `target`: what changes of names, unique values and references,
    /// and the name its `[find ...]` looks for.
    pub fn update<'a, T: RosResource, F: KeyedResource>(
        target: &'a T,
        from: &'a F,
    ) -> MutationOrdering<'a> {
        Self::update_single(target, from, find_reference(from))
    }

    /// Updating `from` into `target` without a key (`is-single`, or found by `.id`).
    pub fn update_single<'a, T: RosResource, F: RosResource>(
        target: &'a T,
        from: &'a F,
        finds: Option<Reference<'a>>,
    ) -> MutationOrdering<'a> {
        let before = unique_values(from).collect::<HashSet<_>>();
        let after = unique_values(target).collect::<HashSet<_>>();
        let still_provided = provided(target).collect::<HashSet<_>>();
        let blocking = target.removal_blocking_references().collect::<HashSet<_>>();
        MutationOrdering {
            releases: provided(from).filter(|r| !still_provided.contains(r)).collect(),
            claims: after.iter().filter(|v| !before.contains(*v)).cloned().collect(),
            frees: before.iter().filter(|v| !after.contains(*v)).cloned().collect(),
            drops: from
                .removal_blocking_references()
                .filter(|(_, value)| !value.is_empty())
                .filter(|r| !blocking.contains(r))
                .collect(),
            finds: finds.into_iter().collect(),
        }
    }

    /// Removing `entry`: it gives up its names, unique values and references.
    pub fn remove<'a, C: KeyedResource>(entry: &'a C) -> MutationOrdering<'a> {
        MutationOrdering {
            releases: provided(entry).collect(),
            claims: Box::new([]),
            frees: unique_values(entry).collect(),
            drops: entry
                .removal_blocking_references()
                .filter(|(_, value)| !value.is_empty())
                .collect(),
            finds: find_reference(entry).into_iter().collect(),
        }
    }
}

/// What an update of `from` into `target` refers to: the references it writes. A `set` sends
/// only the changed values, so a reference the entry keeps is there already (or left dangling
/// by RouterOS) and needs nobody; an update without changes needs nothing at all. A bound
/// radio reports the bridge it gets through its configuration (`datapath.bridge`), which it
/// doesn't refer to itself.
pub fn written_references<'a, T: RosResource, F: RosResource>(
    target: &'a T,
    from: &F,
) -> Box<[Reference<'a>]> {
    let kept = from.consumes_reference().collect::<HashSet<_>>();
    target
        .consumes_reference()
        .filter(|(_, value)| !value.is_empty())
        .filter(|reference| !kept.contains(reference))
        .collect()
}

fn provided<R: RosResource>(entry: &R) -> impl Iterator<Item = Reference<'_>> {
    entry
        .provides_reference()
        .filter(|(_, value)| !value.is_empty())
}

fn unique_values<R: RosResource>(entry: &R) -> impl Iterator<Item = UniqueValue<'_>> {
    entry
        .unique_values()
        .filter(|(_, value)| !value.is_empty())
        .map(|(field, value)| UniqueValue {
            resource: R::path(),
            field,
            value,
        })
}

fn find_reference<K: KeyedResource>(entry: &K) -> Option<Reference<'_>> {
    K::key_reference().map(|reference| (reference, entry.key_value().encode_ros()))
}

#[derive(Clone, PartialEq)]
pub struct ResourceMutation<'a> {
    pub resource: &'static [u8],
    pub operation: ResourceMutationOperation<'a>,
    pub fields: Box<[KeyValuePair<'a>]>,
    pub depends: Box<[Reference<'a>]>,
    pub provides: Box<[Reference<'a>]>,
    pub ordering: MutationOrdering<'a>,
}
impl Debug for ResourceMutation<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ResourceMutation {{resource: {}, operation: {:?}, fields: {:?}, depends: [",
            decode_latin1(self.resource),
            self.operation,
            self.fields
        )?;
        for (idx, (reference, id)) in self.depends.iter().enumerate() {
            if idx > 0 {
                f.write_str(", ")?
            }
            write!(f, "{:?}:{}", reference, decode_latin1(id.as_ref()))?;
        }
        f.write_str("], provides: [")?;
        for (idx, (reference, id)) in self.provides.iter().enumerate() {
            if idx > 0 {
                f.write_str(", ")?
            }
            write!(f, "{:?}:{}", reference, decode_latin1(id.as_ref()))?;
        }
        write!(f, "], ordering: {:?}}}", self.ordering)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct MissingDependenciesError<'a, 'b> {
    pub dependencies: HashSet<(ReferenceType, Cow<'a, [u8]>)>,
    pub unresolved_mutations: Box<[&'b ResourceMutation<'a>]>,
}

impl Display for MissingDependenciesError<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let missing_deps = self
            .dependencies
            .iter()
            .map(|(k, v)| format!("{k:?}:\"{}\"", decode_latin1(v.as_ref())))
            .join(", ");
        let resources: HashSet<_> = self
            .unresolved_mutations
            .iter()
            .map(|d| decode_latin1(d.resource))
            .collect();
        if self.dependencies.is_empty() {
            // e.g. two entries swapping a unique value
            writeln!(f, "Changes waiting on each other on {resources:?}")
        } else {
            writeln!(f, "Missing dependencies: {missing_deps} on {resources:?}")
        }
    }
}

impl<'a> ResourceMutation<'a> {
    pub fn into_owned(self) -> ResourceMutation<'static> {
        let operation = self.operation.into_owned();
        let fields = self
            .fields
            .into_iter()
            .map(|kv| KeyValuePair::into_owned(kv))
            .collect();
        let depends = self
            .depends
            .into_iter()
            .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
            .collect();
        let provides = self
            .provides
            .into_iter()
            .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
            .collect();
        ResourceMutation {
            resource: self.resource,
            operation,
            fields,
            depends,
            provides,
            ordering: self.ordering.into_owned(),
        }
    }
    pub fn sort_mutations<'b, 'c>(
        updates: &'c [ResourceMutation<'b>],
    ) -> Result<Box<[&'c ResourceMutation<'b>]>, MissingDependenciesError<'b, 'c>> {
        Self::sort_mutations_with_provided_dependencies(updates, None)
    }
    /// Orders the mutations so that every one runs when what it needs is there and what it
    /// takes is free:
    /// - what it refers to (`depends`) is provided, by RouterOS or an earlier mutation;
    /// - a name it provides isn't still held: after the removal or rename that releases it;
    /// - a unique value it claims isn't still held: after the mutation that frees it;
    /// - a name it releases isn't still needed: after the mutations that drop a reference to it
    ///   or find an entry by it.
    ///
    /// Mutations that wait on each other (two entries swapping a unique value) or on a name
    /// nobody provides are returned as error.
    pub fn sort_mutations_with_provided_dependencies<'b, 'c>(
        updates: &'c [ResourceMutation<'b>],
        provided_dependencies: impl IntoIterator<Item = (ReferenceType, Cow<'b, [u8]>)>,
    ) -> Result<Box<[&'c ResourceMutation<'b>]>, MissingDependenciesError<'b, 'c>> {
        let mut provided_dependencies = provided_dependencies.into_iter().collect::<HashSet<_>>();
        let mut pending = vec![true; updates.len()];
        let mut sorted_mutations = Vec::with_capacity(updates.len());
        while sorted_mutations.len() < updates.len() {
            let mut could_add = false;
            for (idx, mutation) in updates.iter().enumerate() {
                if !pending[idx] {
                    continue;
                }
                let others = || {
                    updates
                        .iter()
                        .enumerate()
                        .filter(|(other, _)| *other != idx && pending[*other])
                        .map(|(_, m)| m)
                };
                let ready = mutation
                    .depends
                    .iter()
                    .all(|dep| provided_dependencies.contains(dep))
                    && !others().any(|other| mutation.waits_for(other));
                if ready {
                    for dep in &mutation.provides {
                        info!(
                            "{} provides {:?}:{}",
                            decode_latin1(mutation.resource),
                            dep.0,
                            decode_latin1(dep.1.as_ref())
                        );
                        provided_dependencies.insert(dep.clone());
                    }
                    pending[idx] = false;
                    sorted_mutations.push(mutation);
                    could_add = true;
                }
            }
            if !could_add {
                let next_round = updates
                    .iter()
                    .zip(&pending)
                    .filter(|(_, pending)| **pending)
                    .map(|(m, _)| m)
                    .collect::<Vec<_>>();
                let dependencies = next_round
                    .iter()
                    .flat_map(|m| {
                        m.depends
                            .iter()
                            .filter(|dep| !provided_dependencies.contains(dep))
                    })
                    .cloned()
                    .collect();
                return Err(MissingDependenciesError {
                    dependencies,
                    unresolved_mutations: next_round.into_boxed_slice(),
                });
            }
        }
        Ok(sorted_mutations.into_boxed_slice())
    }

    /// Whether `other`, still pending, has to run before this one.
    fn waits_for(&self, other: &ResourceMutation<'_>) -> bool {
        let ordering = &self.ordering;
        let other_ordering = &other.ordering;
        // a name it provides is still held by `other`
        self.provides.iter().any(|name| other_ordering.releases.contains(name))
            // a unique value it claims is still held by `other`
            || ordering.claims.iter().any(|value| other_ordering.frees.contains(value))
            // a name it releases is still needed by `other`
            || ordering.releases.iter().any(|name| {
                other_ordering.drops.contains(name) || other_ordering.finds.contains(name)
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceMutationOperation<'a> {
    Add,
    RemoveByKey(KeyValuePair<'a>),
    UpdateSingle,
    UpdateByKey(KeyValuePair<'a>),
}

impl<'a> ResourceMutationOperation<'a> {
    pub fn into_owned(self) -> ResourceMutationOperation<'static> {
        match self {
            ResourceMutationOperation::Add => ResourceMutationOperation::Add,
            ResourceMutationOperation::RemoveByKey(k) => {
                ResourceMutationOperation::RemoveByKey(k.into_owned())
            }
            ResourceMutationOperation::UpdateSingle => ResourceMutationOperation::UpdateSingle,
            ResourceMutationOperation::UpdateByKey(k) => {
                ResourceMutationOperation::UpdateByKey(k.into_owned())
            }
        }
    }
}

/// Applies a single [`ResourceMutation`] to a live device over the native API.
///
/// Only for disposable test devices right now, not production use: this has
/// no awareness of the managed-marker scheme that will be needed before it's
/// safe to run against a real, already-configured device (it would propose
/// removing/overwriting entries NetBox never created).
///
/// RouterOS's native API only accepts a query filter (`?key=value`) on
/// `/print`; `/set` and `/remove` both require an explicit `.id` (confirmed
/// against a live device - a query-filtered `/set` is rejected with "missing
/// =.id="). `RemoveByKey`/`UpdateByKey` therefore need a `/print` round trip
/// first to resolve the key into a `.id` before the actual write.
pub async fn apply_mutation(
    device: &mikrotik_api::prelude::MikrotikDevice<SimpleResult>,
    mutation: &ResourceMutation<'_>,
) -> Result<(), Error> {
    match &mutation.operation {
        ResourceMutationOperation::Add => {
            write_command(device, mutation.resource, b"add", |cmd| {
                with_fields(cmd, &mutation.fields)
            })
            .await
        }
        ResourceMutationOperation::UpdateSingle => {
            write_command(device, mutation.resource, b"set", |cmd| {
                with_fields(cmd, &mutation.fields)
            })
            .await
        }
        ResourceMutationOperation::RemoveByKey(key) => {
            let id = find_id_by_key(device, mutation.resource, key).await?;
            write_command(device, mutation.resource, b"remove", |cmd| {
                cmd.attribute(b".id".as_slice(), id.as_bytes())
            })
            .await
        }
        ResourceMutationOperation::UpdateByKey(key) => {
            let id = find_id_by_key(device, mutation.resource, key).await?;
            write_command(device, mutation.resource, b"set", |cmd| {
                with_fields(
                    cmd.attribute(b".id".as_slice(), id.as_bytes()),
                    &mutation.fields,
                )
            })
            .await
        }
    }
}

/// Renders `mutations` as a RouterOS script and runs it as an independent,
/// device-local job via `/system/script`, instead of applying each mutation
/// one command at a time over this connection the way [`apply_mutation`]
/// does.
///
/// This is what makes a config change survive the connection breaking
/// mid-apply - including the most severe case, where the change being
/// applied is itself what severs the connection (e.g. reconfiguring the
/// very interface/route this API session is reachable through). Once
/// `/system/script/run` is acknowledged, RouterOS keeps executing the
/// script as its own job, completely decoupled from the API session that
/// started it - confirmed live against a real CHR: a script still runs to
/// completion with the triggering connection already closed and no
/// connection open at all for the rest of its duration.
///
/// Does not wait for the script to finish, and does not implement any
/// rollback-on-failure yet (planned as a separate, later addition: schedule
/// a revert script via `/system/scheduler` before running this one, then
/// cancel it once the device is confirmed reachable again with the new
/// config). A stale `/system/script` entry named `script_name` from a
/// previous, crashed call is removed first (best-effort - "no such item" is
/// the expected case on a first call), so repeated calls with the same name
/// don't drift; the entry itself is intentionally left on the device
/// afterward for operator visibility rather than cleaned up immediately,
/// since this function returns before knowing whether the script actually
/// finished successfully.
pub async fn apply_mutations_via_script(
    device: &mikrotik_api::prelude::MikrotikDevice<SimpleResult>,
    mutations: &[ResourceMutation<'_>],
    script_name: &str,
) -> Result<(), Error> {
    let mut source = String::new();
    let mut generator = crate::generator::Generator::new(&mut source);
    for mutation in mutations {
        generator
            .append_mutation(mutation)
            .expect("writing to a String cannot fail");
    }

    let _ = write_command(device, b"system/script", b"remove", |cmd| {
        cmd.attribute(b"numbers".as_slice(), script_name.as_bytes())
    })
    .await;

    write_command(device, b"system/script", b"add", |cmd| {
        cmd.attribute(b"name".as_slice(), script_name.as_bytes())
            .attribute(b"source".as_slice(), source.as_bytes())
    })
    .await?;

    // answers only once the whole script is through
    let path = b"/system/script/run".as_slice();
    let mut stream = device
        .send_command_with_reply_timeout(
            path,
            |cmd| cmd.attribute(b"number".as_slice(), script_name.as_bytes()),
            (),
            Some(SCRIPT_RUN_TIMEOUT),
        )
        .await;
    while let Some(result) = stream.next().await {
        match result {
            SimpleResult::Sentence(_) => {}
            SimpleResult::Error(e) => return Err(Error::Device(e)),
            SimpleResult::Trap { category, message } => {
                return Err(Error::Trap(TrapResponse {
                    category,
                    message: message.into_boxed_bytes(),
                }));
            }
        }
    }
    Ok(())
}

/// How long the script of [`apply_mutations_via_script`] may run: `/system/script/run` replies
/// only at its end, so the API's usual limit without a reply would be too short.
pub const SCRIPT_RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10 * 60);

fn with_fields<'a>(cmd: CommandBuilder, fields: &[KeyValuePair<'a>]) -> CommandBuilder {
    fields
        .iter()
        .fold(cmd, |cmd, kv| cmd.attribute(kv.key, kv.value.as_ref()))
}

async fn write_command<F: FnOnce(CommandBuilder) -> CommandBuilder>(
    device: &mikrotik_api::prelude::MikrotikDevice<SimpleResult>,
    resource: &'static [u8],
    verb: &'static [u8],
    command_builder: F,
) -> Result<(), Error> {
    let path = [b"/".as_slice(), resource, b"/".as_slice(), verb].concat();
    let mut stream = device
        .send_command(path.as_slice(), command_builder, ())
        .await;
    while let Some(result) = stream.next().await {
        match result {
            SimpleResult::Sentence(_) => {}
            SimpleResult::Error(e) => return Err(Error::Device(e)),
            SimpleResult::Trap { category, message } => {
                return Err(Error::Trap(TrapResponse {
                    category,
                    message: message.into_boxed_bytes(),
                }));
            }
        }
    }
    Ok(())
}

async fn find_id_by_key(
    device: &mikrotik_api::prelude::MikrotikDevice<SimpleResult>,
    resource: &'static [u8],
    key: &KeyValuePair<'_>,
) -> Result<Box<str>, Error> {
    let path = [b"/".as_slice(), resource, b"/print".as_slice()].concat();
    let mut stream = device
        .send_command(
            path.as_slice(),
            |cmd| cmd.query_equal(key.key, key.value.as_ref()),
            (),
        )
        .await;
    while let Some(result) = stream.next().await {
        match result {
            SimpleResult::Sentence(mut row) => {
                if let Some(Some(id)) = row.remove(".id") {
                    return Ok(id);
                }
            }
            SimpleResult::Error(e) => return Err(Error::Device(e)),
            SimpleResult::Trap { category, message } => {
                return Err(Error::Trap(TrapResponse {
                    category,
                    message: message.into_boxed_bytes(),
                }));
            }
        }
    }
    Err(Error::KeyNotFound {
        resource,
        key: key.key,
        value: key.value.clone().into_owned().into_boxed_slice(),
    })
}

pub trait Updatable<From: RosResource>: DeserializeRosResource {
    fn calculate_update<'a>(&'a self, from: &'a From) -> ResourceMutation<'a>;
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
#[derive(Debug, Clone)]
pub enum SentenceResult<R> {
    Row {
        value: R,
        warnings: Arc<Box<[ResourceAccessWarning]>>,
    },
    Error {
        errors: Arc<Box<[ResourceAccessError]>>,
        warnings: Arc<Box<[ResourceAccessWarning]>>,
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
                warnings: warnings.clone(),
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
                    warnings: Arc::new(warnings.into_boxed_slice()),
                },
                Err(field_name) => {
                    errors.push(ResourceAccessError::UndefinedFieldError { field_name });
                    SentenceResult::Error {
                        errors: Arc::new(errors.into_boxed_slice()),
                        warnings: Arc::new(warnings.into_boxed_slice()),
                    }
                }
            }
        } else {
            SentenceResult::Error {
                errors: Arc::new(errors.into_boxed_slice()),
                warnings: Arc::new(warnings.into_boxed_slice()),
            }
        }
    }

    fn process_error(error: &mikrotik_api::error::Error, _context: &Self::Context) -> Self {
        SentenceResult::Error {
            errors: Arc::new(Box::new([ResourceAccessError::ApiError(error.clone())])),
            warnings: Arc::default(),
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
                    warnings: Arc::new(warnings.into_boxed_slice()),
                },
                Err(field_name) => {
                    errors.push(ResourceAccessError::MissingFieldError { field_name });
                    SentenceResult::Error {
                        errors: Arc::new(errors.into_boxed_slice()),
                        warnings: Arc::new(warnings.into_boxed_slice()),
                    }
                }
            }
        } else {
            SentenceResult::Error {
                errors: Arc::new(errors.into_boxed_slice()),
                warnings: Arc::new(warnings.into_boxed_slice()),
            }
        }
    }

    fn process_error(error: &mikrotik_api::error::Error, _context: &Self::Context) -> Self {
        SentenceResult::Error {
            errors: Arc::new(Box::new([ResourceAccessError::ApiError(error.clone())])),
            warnings: Arc::default(),
        }
    }

    fn process_trap(result: TrapResult, _context: &Self::Context) -> Self {
        SentenceResult::Trap {
            category: result.category,
            message: Box::from(result.message),
        }
    }
}
#[derive(Debug, Error)]
pub enum ResourceMutationError {
    #[error("cannot insert {entry:?}")]
    Add { entry: Box<Resource> },
    #[error("cannot remove {entry:?}")]
    Remove { entry: Box<Resource> },
    #[error("wrong device type found, expected {expected:?} but got {actual:?} instead")]
    WrongDeviceType {
        expected: DeviceType,
        actual: AsciiString,
    },
}

#[derive(Debug)]
pub struct UpdatePairing<'c, 't, Current: RosResource, Target: CfgResource + Clone> {
    pub orphaned_entries: Box<[&'c Current]>,
    pub matched_entries: Box<[(&'c Current, Cow<'t, Target>)]>,
    pub new_entries: Box<[Cow<'t, Target>]>,
}

impl<'t, Current, Target> UpdatePairing<'_, 't, Current, Target>
where
    Target: KeyedResource + Clone + CfgResource + Updatable<Current>,
    Current: KeyedResource + Clone,
    Current::Value: 'static + Sized,
{
    pub fn generate_updates_or_error<'r, 's>(
        &'s self,
    ) -> impl Iterator<Item = Result<ResourceMutation<'r>, ResourceMutationError>>
    where
        't: 'r,
        's: 'r,
    {
        self.orphaned_entries
            .iter()
            .cloned()
            .map(Current::create_resource_ref)
            .map(|e| e.cloned())
            .map(|entry| ResourceMutationError::Remove { entry: Box::new(entry) })
            .map(Err)
            .chain(
                self.new_entries
                    .iter()
                    .map(|v| v.as_ref())
                    .map(|r| r.create_resource_ref())
                    .map(|e| e.cloned())
                    .map(|entry| ResourceMutationError::Add { entry: Box::new(entry) })
                    .map(Err),
            )
            .chain(
                self.matched_entries
                    .iter()
                    .map(|(original, target)| target.calculate_update(*original))
                    .map(Ok),
            )
    }
    pub fn generate_update<'r, 's>(
        &'s self,
    ) -> Result<impl Iterator<Item = ResourceMutation<'r>>, ResourceMutationError>
    where
        't: 'r,
        's: 'r,
    {
        if let Some(entry) = self
            .orphaned_entries
            .iter()
            .next()
            .cloned()
            .map(Current::create_resource_ref)
            .map(|e| e.cloned())
        {
            Err(ResourceMutationError::Remove { entry: Box::new(entry) })
        } else if let Some(entry) = self
            .new_entries
            .iter()
            .map(|v| v.as_ref())
            .next()
            .map(|r| r.create_resource_ref())
            .map(|e| e.cloned())
        {
            Err(ResourceMutationError::Add { entry: Box::new(entry) })
        } else {
            Ok(self
                .matched_entries
                .iter()
                .map(|(original, target)| target.calculate_update(*original)))
        }
    }
}

impl<'c, 't, Current, Target> UpdatePairing<'c, 't, Current, Target>
where
    Target: KeyedResource + Creatable + Updatable<Current> + Clone,
    Current: KeyedResource,
    Current::Value: 'static + Sized,
{
    pub fn generate_remove_update_add<'r>(self) -> impl Iterator<Item = ResourceMutation<'r>>
    where
        'c: 'r,
        't: 'r,
    {
        self.orphaned_entries
            .into_iter()
            .map(|entry| {
                let value = entry.key_value().encode_ros();
                let key = Current::key_name();
                ResourceMutation {
                    resource: Current::path(),
                    operation: ResourceMutationOperation::RemoveByKey(KeyValuePair { key, value }),
                    fields: Box::new([]),
                    depends: Default::default(),
                    provides: Default::default(),
                    ordering: MutationOrdering::remove(entry),
                }
            })
            .chain(
                self.matched_entries
                    .into_iter()
                    .map(|(original, target)| match target {
                        Cow::Borrowed(t) => t.calculate_update(original),
                        Cow::Owned(t) => t.calculate_update(original).into_owned(),
                    }),
            )
            .chain(self.new_entries.into_iter().map(|entry| match entry {
                Cow::Borrowed(entry) => entry.calculate_create(),
                Cow::Owned(entry) => entry.calculate_create().into_owned(),
            }))
    }
}
pub fn generate_single_update<'c, 't, 'r, Resource>(
    current: &'c Resource,
    target: &'t Resource,
) -> ResourceMutation<'r>
where
    'c: 'r,
    't: 'r,
    Resource: SingleResource + CfgResource + Clone,
{
    let fields = target
        .changed_values(current)
        //.map(|e| e.into_owned())
        .collect::<Box<[_]>>();
    let provides = target
        .provides_reference()
        //.map(|(k, v)| (k, Cow::Owned(v.into_owned())))
        .collect();
    let depends = written_references(target, current);

    ResourceMutation {
        resource: Resource::path(),
        operation: ResourceMutationOperation::UpdateSingle,
        fields,
        depends,
        provides,
        ordering: MutationOrdering::update_single(target, current, None),
    }
}

/// Pairs entries without a key of their own (only a `.id`) with the targets: the pair needing
/// the fewest changed fields first, so an entry is updated where it already matches best instead
/// of by position. By position, an unrelated entry could be rewritten into one that is still
/// there, e.g. a bridge port moved onto an interface another port still holds, which RouterOS
/// refuses. Unpaired entries are removed (first, so they free what they hold), unpaired targets
/// added.
pub fn generate_add_update_remove_by_id<'c, 't, 'r, Target, Current, TargetIter, IntoTarget>(
    current: &'c [Current],
    target: TargetIter,
) -> impl Iterator<Item = ResourceMutation<'r>> + 'r
where
    'c: 'r,
    't: 'r,
    Current: KeyedResource,
    <Current as KeyedResource>::Value: 'static,
    Target: RosResource + CfgResource + Creatable + Updatable<Current> + Clone + 't,
    TargetIter: IntoIterator<Item = IntoTarget>,
    IntoTarget: Into<Cow<'t, Target>>,
    <TargetIter as IntoIterator>::IntoIter: 'r,
{
    let targets = target.into_iter().map(Into::into).collect::<Vec<Cow<'t, Target>>>();
    let mut costs = Vec::with_capacity(current.len() * targets.len());
    for (c, current) in current.iter().enumerate() {
        let blocking = current.removal_blocking_references().collect::<HashSet<_>>();
        for (t, target) in targets.iter().enumerate() {
            // An entry doesn't move to another entry its `blocks-removal` references name (a
            // vtep to another VXLAN): it is replaced. The removal needs nothing, while the move
            // would hold the old one in place until the new one is there, which may wait for
            // the old one to go (the VNI they share).
            if target.removal_blocking_references().collect::<HashSet<_>>() != blocking {
                continue;
            }
            // An entry keeping its unique values is preferred: one taking another's has to wait
            // for it, and two swapping them can't be ordered at all (bridge ports trading their
            // bridges: each set to the other's interface).
            let update = target.calculate_update(current);
            costs.push(((update.ordering.claims.len(), update.fields.len()), c, t));
        }
    }
    // stable: on a tie the earlier entry, so an unchanged list keeps its order
    costs.sort_by_key(|&(cost, c, t)| (cost, c, t));
    let mut current_paired = vec![None; current.len()];
    let mut target_paired = vec![false; targets.len()];
    for (_, c, t) in costs {
        if current_paired[c].is_none() && !target_paired[t] {
            current_paired[c] = Some(t);
            target_paired[t] = true;
        }
    }
    let mut targets = targets.into_iter().map(Some).collect::<Vec<_>>();
    let removes = current
        .iter()
        .zip(&current_paired)
        .filter(|(_, paired)| paired.is_none())
        .map(|(current, _)| ResourceMutation {
            resource: Current::path(),
            operation: ResourceMutationOperation::RemoveByKey(KeyValuePair {
                key: Current::key_name(),
                value: current.key_value().encode_ros(),
            }),
            fields: Box::new([]),
            depends: Box::new([]),
            provides: Box::new([]),
            ordering: MutationOrdering::remove(current),
        })
        .collect::<Vec<_>>();
    let updates = current
        .iter()
        .zip(&current_paired)
        .filter_map(|(current, paired)| Some((current, targets[(*paired)?].take()?)))
        .map(|(current, target)| match target {
            Cow::Borrowed(t) => t.calculate_update(current),
            Cow::Owned(t) => t.calculate_update(current).into_owned(),
        })
        .collect::<Vec<_>>();
    let creates = targets
        .into_iter()
        .flatten()
        .map(|target| match target {
            Cow::Borrowed(t) => t.calculate_create(),
            Cow::Owned(t) => t.calculate_create().into_owned(),
        })
        .collect::<Vec<_>>();
    removes.into_iter().chain(updates).chain(creates)
}

pub fn generate_add_update_remove_by_key<'c, 't, 'r, Target, Current>(
    current: &'c [Current],
    target: impl IntoIterator<Item = impl Into<Cow<'t, Target>>>,
) -> impl Iterator<Item = ResourceMutation<'r>> + 'r
where
    'c: 'r,
    't: 'r,
    Current: KeyedResource,
    <Current as KeyedResource>::Value: 'static,
    Target: KeyedResource + CfgResource + Creatable + Updatable<Current> + Clone + 't,
    <Target as KeyedResource>::Key: PartialEq<<Current as KeyedResource>::Key>,
{
    UpdatePairing::match_updates_by_key(current, target).generate_remove_update_add()
}
/// [`generate_add_update_remove_by_key`] for a menu where RouterOS refuses to remove an
/// entry of its own (the first of `/ipv6/nd`, `default=yes`) but lets its key be changed: the
/// entries the target no longer has take the ones it adds, in the device's order (its own
/// entry first), by a `set` of the key.
pub fn generate_add_update_remove_by_key_reusing<'c, 't, 'r, Target, Current>(
    current: &'c [Current],
    target: impl IntoIterator<Item = impl Into<Cow<'t, Target>>>,
) -> impl Iterator<Item = ResourceMutation<'r>> + 'r
where
    'c: 'r,
    't: 'r,
    Current: KeyedResource,
    <Current as KeyedResource>::Value: 'static,
    Target: KeyedResource + CfgResource + Creatable + Updatable<Current> + Clone + 't,
    <Target as KeyedResource>::Key: PartialEq<<Current as KeyedResource>::Key>,
{
    let UpdatePairing {
        orphaned_entries,
        matched_entries,
        new_entries,
    } = UpdatePairing::match_updates_by_key(current, target);
    let reused = orphaned_entries.len().min(new_entries.len());
    let mut new_entries = new_entries.into_vec();
    let mut matched_entries = matched_entries.into_vec();
    matched_entries.extend(orphaned_entries.iter().copied().zip(new_entries.drain(..reused)));
    UpdatePairing {
        orphaned_entries: orphaned_entries[reused..].into(),
        matched_entries: matched_entries.into(),
        new_entries: new_entries.into(),
    }
    .generate_remove_update_add()
}
pub fn generate_update_by_key<'c, 't, 'r, 'e, Target, Current>(
    current: &'c [Current],
    target: impl IntoIterator<Item = impl Into<Cow<'t, Target>>>,
) -> Result<impl Iterator<Item = ResourceMutation<'r>> + 'r, ResourceMutationError>
where
    'c: 'r + 'e,
    't: 'r + 'e,
    Current: KeyedResource + Debug,
    <Current as KeyedResource>::Value: 'static,
    Target: KeyedResource + CfgResource + Updatable<Current> + 't + Debug + Clone,
    <Target as KeyedResource>::Key: PartialEq<<Current as KeyedResource>::Key>,
{
    let mut target_refs = target
        .into_iter()
        .map(|e| Into::into(e))
        .collect::<Vec<_>>();
    let mut matched = Vec::with_capacity(current.len().max(target_refs.len()));
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
            return Err(ResourceMutationError::Remove {
                entry: Box::new(c.create_resource_ref().cloned()),
            });
        }
    }
    if let Some(entry) = target_refs.into_iter().next() {
        let resource_ref = entry.create_resource_ref().cloned();
        return Err(ResourceMutationError::Add {
            entry: Box::new(resource_ref),
        });
    }
    Ok(matched.into_iter().map(|(original, target)| match target {
        Cow::Borrowed(t) => t.calculate_update(original),
        Cow::Owned(t) => t.calculate_update(original).into_owned(),
    }))
}

impl<'a, 'b, Target: KeyedResource + CfgResource, Current: KeyedResource>
    UpdatePairing<'a, 'b, Current, Target>
where
    <Target as KeyedResource>::Key: PartialEq<<Current as KeyedResource>::Key>,
    Target: Clone,
{
    pub fn match_updates_by_key(
        current: &'a [Current],
        target: impl IntoIterator<Item = impl Into<Cow<'b, Target>>>,
    ) -> Self {
        let mut orphans = Vec::with_capacity(current.len());
        let mut target_refs = target
            .into_iter()
            .map(|t| Into::into(t))
            .collect::<Vec<_>>();
        let mut matched = Vec::with_capacity(current.len().max(target_refs.len()));
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
impl<'c, 't, Resource> UpdatePairing<'c, 't, Resource, Resource>
where
    Resource: SingleResource + CfgResource + Clone,
{
    pub fn create_pair(current: &'c Resource, target: &'t Resource) -> Self {
        UpdatePairing {
            orphaned_entries: Box::new([]),
            matched_entries: Box::new([(current, Cow::Borrowed(target))]),
            new_entries: Box::new([]),
        }
    }
    pub fn generate_single_update<'r>(self) -> impl Iterator<Item = ResourceMutation<'r>>
    where
        'c: 'r,
        't: 'r,
    {
        self.matched_entries.into_iter().map(|(original, target)| {
            let fields = target
                .changed_values(original)
                .map(|e| e.into_owned())
                .collect::<Box<[_]>>();
            let provides = target
                .provides_reference()
                .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
                .collect();
            let depends = written_references(target.as_ref(), original)
                .into_iter()
                .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
                .collect();

            ResourceMutation {
                resource: Resource::path(),
                operation: ResourceMutationOperation::UpdateSingle,
                fields,
                depends,
                provides,
                ordering: MutationOrdering::update_single(target.as_ref(), original, None)
                    .into_owned(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mutation(operation: ResourceMutationOperation<'static>) -> ResourceMutation<'static> {
        ResourceMutation {
            resource: b"interface/wifi/datapath",
            operation,
            fields: Box::new([]),
            depends: Box::new([]),
            provides: Box::new([]),
            ordering: MutationOrdering::default(),
        }
    }
    fn remove(key: &'static [u8], value: &'static [u8]) -> ResourceMutation<'static> {
        mutation(ResourceMutationOperation::RemoveByKey(KeyValuePair {
            key,
            value: value.into(),
        }))
    }
    fn vni(value: &'static [u8]) -> UniqueValue<'static> {
        UniqueValue {
            resource: b"interface/vxlan",
            field: b"vni",
            value: value.into(),
        }
    }
    fn interface(name: &'static [u8]) -> (ReferenceType, Cow<'static, [u8]>) {
        (ReferenceType::Interface, name.into())
    }
    fn sorted(mutations: &[ResourceMutation<'static>]) -> Vec<ResourceMutation<'static>> {
        ResourceMutation::sort_mutations(mutations)
            .unwrap()
            .iter()
            .map(|m| (*m).clone())
            .collect()
    }

    #[test]
    fn removal_runs_after_updates_that_drop_references() {
        let removal = ResourceMutation {
            ordering: MutationOrdering {
                releases: Box::new([(ReferenceType::WifiDatapath, b"bridge-caps".into())]),
                ..Default::default()
            },
            ..remove(b"name", b"bridge-caps")
        };
        let plain_removal = remove(b".id", b"*1");
        let update = ResourceMutation {
            resource: b"interface/wifi",
            fields: Box::new([KeyValuePair {
                key: b"!datapath",
                value: b"".into(),
            }]),
            ordering: MutationOrdering {
                drops: Box::new([(ReferenceType::WifiDatapath, b"bridge-caps".into())]),
                ..Default::default()
            },
            ..mutation(ResourceMutationOperation::UpdateByKey(KeyValuePair {
                key: b"default-name",
                value: b"wifi1".into(),
            }))
        };
        let mutations = [removal.clone(), plain_removal.clone(), update.clone()];
        assert_eq!(sorted(&mutations), vec![plain_removal, update, removal]);
    }

    /// RouterOS refuses a second VXLAN with the same VNI, so the one holding it goes first.
    #[test]
    fn claim_of_a_unique_value_waits_for_its_release() {
        let add = ResourceMutation {
            resource: b"interface/vxlan",
            provides: Box::new([interface(b"vxlan-new")]),
            ordering: MutationOrdering {
                claims: Box::new([vni(b"1")]),
                ..Default::default()
            },
            ..mutation(ResourceMutationOperation::Add)
        };
        let removal = ResourceMutation {
            resource: b"interface/vxlan",
            ordering: MutationOrdering {
                releases: Box::new([interface(b"vxlan-old")]),
                frees: Box::new([vni(b"1")]),
                ..Default::default()
            },
            ..remove(b"name", b"vxlan-old")
        };
        let mutations = [add.clone(), removal.clone()];
        assert_eq!(sorted(&mutations), vec![removal, add]);
    }

    /// A name given up by a rename can be taken by another entry afterwards, and a
    /// `[find interface=X]` has to run while X is still there.
    #[test]
    fn released_name_is_taken_and_found_in_order() {
        let rename = ResourceMutation {
            resource: b"interface/vxlan",
            fields: Box::new([KeyValuePair {
                key: b"name",
                value: b"vx-b".into(),
            }]),
            provides: Box::new([interface(b"vx-b")]),
            ordering: MutationOrdering {
                releases: Box::new([interface(b"vx-a")]),
                ..Default::default()
            },
            ..mutation(ResourceMutationOperation::UpdateByKey(KeyValuePair {
                key: b"name",
                value: b"vx-a".into(),
            }))
        };
        let add = ResourceMutation {
            resource: b"interface/vlan",
            provides: Box::new([interface(b"vx-a")]),
            ..mutation(ResourceMutationOperation::Add)
        };
        let find = ResourceMutation {
            resource: b"ipv6/nd",
            ordering: MutationOrdering {
                finds: Box::new([interface(b"vx-a")]),
                ..Default::default()
            },
            ..remove(b"interface", b"vx-a")
        };
        let mutations = [add.clone(), rename.clone(), find.clone()];
        assert_eq!(sorted(&mutations), vec![find, rename, add]);
    }

    /// Two entries swapping their VNIs wait on each other: an error, not a script that fails.
    #[test]
    fn swapping_unique_values_is_an_error() {
        let swap = |name: &'static [u8], from: &'static [u8], to: &'static [u8]| ResourceMutation {
            resource: b"interface/vxlan",
            ordering: MutationOrdering {
                claims: Box::new([vni(to)]),
                frees: Box::new([vni(from)]),
                ..Default::default()
            },
            ..mutation(ResourceMutationOperation::UpdateByKey(KeyValuePair {
                key: b"name",
                value: name.into(),
            }))
        };
        let mutations = [swap(b"vx-a", b"1", b"2"), swap(b"vx-b", b"2", b"1")];
        let error = ResourceMutation::sort_mutations(&mutations).unwrap_err();
        assert!(error.dependencies.is_empty());
        assert_eq!(error.unresolved_mutations.len(), 2);
    }

    /// ap-klr-solar: RouterOS's own entry (`*0`, `default=yes`) was set to `interface=switch`;
    /// `remove *0` fails ("can not remove default rule"), so it is set to the target's
    /// `all` instead
    #[test]
    fn own_nd_entry_is_set_instead_of_removed() {
        use crate::model::{Ipv6NdByInterface, Ipv6NdCfg};
        let current = [Ipv6NdByInterface(Ipv6NdCfg::new(b"switch".into()))];
        let target = [Ipv6NdByInterface(Ipv6NdCfg::new(b"all".into()))];
        let mutations =
            generate_add_update_remove_by_key_reusing(&current, target.iter().map(Cow::Borrowed))
                .collect::<Vec<_>>();
        assert_eq!(mutations.len(), 1);
        assert_eq!(
            mutations[0].operation,
            ResourceMutationOperation::UpdateByKey(KeyValuePair {
                key: b"interface",
                value: b"switch".as_slice().into(),
            })
        );
        assert_eq!(
            mutations[0].fields.as_ref(),
            [KeyValuePair {
                key: b"interface",
                value: b"all".as_slice().into(),
            }]
        );
    }
}
