//! The commands of the modeled menus, for `Connection::run`: `print` and `monitor once` read as
//! the menu's model, and the writes of an apply. Actions without a menu of their own belong
//! in a catalog beside these.
use crate::{
    model::Resource,
    resource::{
        DeserializeRosResource, KeyedResource, KeyedRow, KeyedRowContext, RosResource,
        SentenceResult, written_as,
    },
    value::{KeyValuePair, RosValue},
};
use mikrotik_api::{
    command::{Observe, Read, Write},
    prelude::{Command, CommandBuilder, Connection, ParsedMessage, QueryOperator, Replies},
    simple::SimpleResult,
};
use std::{borrow::Cow, future::Future, marker::PhantomData, time::Duration};

/// What may read from a device: commands that change nothing ([`Read`], [`Observe`]). A
/// [`Connection`] may, as may a facade of the user that may do no more (the provisioner's
/// `Reader`).
pub trait Reads: Sync {
    fn read<C: Command<Effect = Read> + Sync>(
        &self,
        command: &C,
    ) -> impl Future<Output = Replies<C::Reply>> + Send;
    fn observe<C: Command<Effect = Observe> + Sync>(
        &self,
        command: &C,
    ) -> impl Future<Output = Replies<C::Reply>> + Send;
}

/// What may also change a device ([`Write`]): the writes of an apply, the actions of the
/// catalog.
pub trait Writes: Reads {
    fn write<C: Command<Effect = Write> + Sync>(
        &self,
        command: &C,
    ) -> impl Future<Output = Replies<C::Reply>> + Send;
}

impl Reads for Connection {
    fn read<C: Command<Effect = Read> + Sync>(
        &self,
        command: &C,
    ) -> impl Future<Output = Replies<C::Reply>> + Send {
        self.run(command)
    }
    fn observe<C: Command<Effect = Observe> + Sync>(
        &self,
        command: &C,
    ) -> impl Future<Output = Replies<C::Reply>> + Send {
        self.run(command)
    }
}

impl Writes for Connection {
    fn write<C: Command<Effect = Write> + Sync>(
        &self,
        command: &C,
    ) -> impl Future<Output = Replies<C::Reply>> + Send {
        self.run(command)
    }
}

fn menu_path(resource: &[u8], verb: &[u8]) -> Cow<'static, [u8]> {
    Cow::Owned([b"/", resource, b"/", verb].concat())
}

/// `print` of `R`'s menu, every entry.
pub struct Print<R>(PhantomData<fn() -> R>);

impl<R> Default for Print<R> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<R: DeserializeRosResource + RosResource> Command for Print<R> {
    type Effect = Read;
    type Reply = SentenceResult<Resource>;
    fn path(&self) -> Cow<'_, [u8]> {
        menu_path(R::path(), b"print")
    }
    fn context(&self) -> <Self::Reply as ParsedMessage>::Context {
        R::resource_type()
    }
}

/// `print` of the entries of `R`'s menu that have its key, or the one with `key`: the whole
/// menu is asked and the rows are judged here ([`KeyedRow`]), never by a query on the device.
pub struct PrintKeyed<'k, R: KeyedResource> {
    key: Option<&'k R::Key>,
}

impl<'k, R: KeyedResource> PrintKeyed<'k, R> {
    pub fn all() -> Self {
        Self { key: None }
    }
    pub fn with(key: &'k R::Key) -> Self {
        Self { key: Some(key) }
    }
}

impl<R: KeyedResource> Command for PrintKeyed<'_, R> {
    type Effect = Read;
    type Reply = KeyedRow;
    fn path(&self) -> Cow<'_, [u8]> {
        menu_path(R::path(), b"print")
    }
    fn context(&self) -> <Self::Reply as ParsedMessage>::Context {
        KeyedRowContext {
            resource: R::resource_type(),
            key_name: R::key_name(),
            key: self.key.map(|key| key.encode_ros().into()),
            written: written_as::<R::Key>,
            entries: R::entries(),
        }
    }
}

/// `monitor` of the entries `ids` (comma separated) of `R`'s menu, once.
pub struct MonitorOnce<R> {
    ids: Box<[u8]>,
    resource: PhantomData<fn() -> R>,
}

impl<R> MonitorOnce<R> {
    pub fn new(ids: Box<[u8]>) -> Self {
        Self {
            ids,
            resource: PhantomData,
        }
    }
}

impl<R: DeserializeRosResource + RosResource> Command for MonitorOnce<R> {
    type Effect = Observe;
    type Reply = SentenceResult<Resource>;
    fn path(&self) -> Cow<'_, [u8]> {
        menu_path(R::path(), b"monitor")
    }
    fn build(&self, builder: CommandBuilder) -> CommandBuilder {
        builder
            .attribute(b".id", self.ids.as_ref())
            .attribute(b"once", b"true")
    }
    fn context(&self) -> <Self::Reply as ParsedMessage>::Context {
        R::resource_type()
    }
}

/// A write to the menu `resource` (`add`, `set`, `remove`, `move`) with its attributes.
pub(crate) struct MenuWrite<'a> {
    pub resource: &'static [u8],
    pub verb: &'static [u8],
    pub attributes: &'a [KeyValuePair<'a>],
    pub reply_timeout: Option<Duration>,
}

impl Command for MenuWrite<'_> {
    type Effect = Write;
    type Reply = SimpleResult;
    fn path(&self) -> Cow<'_, [u8]> {
        menu_path(self.resource, self.verb)
    }
    fn build(&self, builder: CommandBuilder) -> CommandBuilder {
        self.attributes.iter().fold(builder, |builder, kv| {
            builder.attribute(kv.key, kv.value.as_ref())
        })
    }
    fn context(&self) {}
    fn reply_timeout(&self) -> Option<Duration> {
        self.reply_timeout
    }
}

/// `print` of the entry of the menu `resource` with `key`, for its `.id`; a static one in a
/// menu that has dynamic entries, as the script's `[find ... dynamic!=yes]`.
pub(crate) struct FindByKey<'a> {
    pub resource: &'static [u8],
    pub key: &'a KeyValuePair<'a>,
}

impl Command for FindByKey<'_> {
    type Effect = Read;
    type Reply = SimpleResult;
    fn path(&self) -> Cow<'_, [u8]> {
        menu_path(self.resource, b"print")
    }
    fn build(&self, builder: CommandBuilder) -> CommandBuilder {
        let builder = builder.query_equal(self.key.key, self.key.value.as_ref());
        // as the script's `[find ... dynamic!=yes]` (`generator::static_only`): an entry of a
        // menu without the property (`/routing/ospf/area`) counts as static
        if crate::model::finds_static_only(self.resource) {
            builder
                .query_equal(b"dynamic", b"false")
                .query_not_present(b"dynamic")
                .query_operations([QueryOperator::Or, QueryOperator::And].into_iter())
        } else {
            builder
        }
    }
    fn context(&self) {}
}
