use crate::resource::{Creatable, CreateHandler, ResourceMutation, ResourceMutationOperation};
use crate::value::{KeyValuePair, write_script_string};
use encoding_rs::mem::decode_latin1;
use std::fmt::Write;

#[derive(Debug)]
pub struct Generator<'a, W: Write> {
    target: &'a mut W,
    current_path: Option<&'static [u8]>,
}

impl<'a, W: Write> CreateHandler<()> for &mut Generator<'a, W> {
    fn handle_creatable<T: Creatable>(self, value: &T) {
        Creatable::create(value, self);
    }
}

impl<'a, W: Write> Generator<'a, W> {
    pub fn new(target: &'a mut W) -> Self {
        Self {
            target,
            current_path: None,
        }
    }
    pub fn append_mutation(&mut self, mutation: &ResourceMutation) -> std::fmt::Result {
        match &mutation.operation {
            ResourceMutationOperation::RemoveByKey(_) | ResourceMutationOperation::Move { .. } => {}
            _ => {
                if mutation.fields.is_empty() {
                    return Ok(());
                }
            }
        }
        if Some(mutation.resource) != self.current_path {
            writeln!(self.target, "/{}", decode_latin1(mutation.resource))?;
            self.current_path = Some(mutation.resource);
        }
        match &mutation.operation {
            ResourceMutationOperation::Add => {
                write!(self.target, "add ")?;
                self.append_fields(&mutation.fields)?;
                writeln!(self.target)?;
            }
            ResourceMutationOperation::RemoveByKey(id_key) => {
                self.target.write_str("remove [find ")?;
                self.append_field(id_key)?;
                self.target.write_str(static_only(mutation))?;
                self.target.write_str("]\n")?;
            }
            ResourceMutationOperation::UpdateSingle => {
                self.target.write_str("set ")?;
                self.append_fields(&mutation.fields)?;
                writeln!(self.target)?;
            }
            ResourceMutationOperation::UpdateByKey(id_key) => {
                self.target.write_str("set [ find ")?;
                self.append_field(id_key)?;
                self.target.write_str(static_only(mutation))?;
                self.target.write_str("] ")?;
                self.append_fields(&mutation.fields)?;
                writeln!(self.target)?;
            }
            ResourceMutationOperation::Move { entries, before, .. } => {
                for entry in entries {
                    self.target.write_str("move [find ")?;
                    self.append_field(entry)?;
                    self.target.write_str(static_only(mutation))?;
                    self.target.write_str("]")?;
                    if let Some(before) = before {
                        write!(self.target, " destination={}", decode_latin1(before))?;
                    }
                    writeln!(self.target)?;
                }
            }
        }
        Ok(())
    }
    fn append_fields(&mut self, mutation: &[KeyValuePair]) -> std::fmt::Result {
        for kv in mutation {
            self.append_field(kv)?;
            self.target.write_char(' ')?
        }
        Ok(())
    }
    fn append_field(&mut self, kv: &KeyValuePair) -> std::fmt::Result {
        // `!field` clears the field, it has no value
        if kv.key.starts_with(b"!") {
            return write!(self.target, "{}", decode_latin1(kv.key));
        }
        write!(self.target, "{}=", decode_latin1(kv.key))?;
        if !kv.value.is_empty()
            && kv.value.iter().copied().all(|ch| {
                ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' || ch == b',' || ch == b'*'
            })
        {
            write!(self.target, "{}", decode_latin1(kv.value.as_ref()))?;
        } else {
            write_script_string(self.target, &kv.value)?;
        }
        Ok(())
    }
}

/// What a `[find ...]` adds in a menu read without its dynamic entries
/// ([`crate::model::finds_static_only`]): an open connection of `/ip/service` has the name
/// `api` too, and RouterOS refuses a `set` that finds it. `dynamic!=yes`, not `dynamic=no`: an
/// OSPF area has no `dynamic` at all, `dynamic=no` found none and the `set` did nothing
/// (sat-switch 7.23.8, CHR 7.24.5); `!dynamic` is a syntax error in `/ipv6/address`.
fn static_only(mutation: &ResourceMutation) -> &'static str {
    if crate::model::finds_static_only(mutation.resource) { " dynamic!=yes" } else { "" }
}

pub fn generate_cfg(target: &mut impl Write, mutations: &[ResourceMutation]) -> std::fmt::Result {
    let mut current_path = None;
    for mutation in mutations {
        if Some(mutation.resource) != current_path {
            writeln!(target, "/{},", decode_latin1(mutation.resource))?;
            current_path = Some(mutation.resource);
        }
        match &mutation.operation {
            ResourceMutationOperation::Add => {
                write!(target, "add ")?;
                append_fields(target, mutation)?;
                writeln!(target)?;
            }
            ResourceMutationOperation::RemoveByKey(id_key) => {
                target.write_str("remove [find ")?;
                append_field(target, id_key)?;
                target.write_str(static_only(mutation))?;
                target.write_str("]\n")?;
            }
            ResourceMutationOperation::UpdateSingle => {
                target.write_str("set ")?;
                append_fields(target, mutation)?;
                writeln!(target)?;
            }
            ResourceMutationOperation::UpdateByKey(id_key) => {
                target.write_str("set [ find ")?;
                append_field(target, id_key)?;
                target.write_str(static_only(mutation))?;
                target.write_str("] ")?;
                append_fields(target, mutation)?;
                writeln!(target)?;
            }
            ResourceMutationOperation::Move { entries, before, .. } => {
                for entry in entries {
                    target.write_str("move [find ")?;
                    append_field(target, entry)?;
                    target.write_str(static_only(mutation))?;
                    target.write_str("]")?;
                    if let Some(before) = before {
                        write!(target, " destination={}", decode_latin1(before))?;
                    }
                    writeln!(target)?;
                }
            }
        }
    }
    Ok(())
}

fn append_fields<W: Write>(target: &mut W, mutation: &ResourceMutation) -> std::fmt::Result {
    for kv in &mutation.fields {
        append_field(target, kv)?;
        target.write_char(' ')?
    }
    Ok(())
}

fn append_field(target: &mut impl Write, kv: &KeyValuePair) -> std::fmt::Result {
    write!(target, "{}=", decode_latin1(kv.key))?;
    if !kv.value.is_empty()
        && kv
            .value
            .iter()
            .copied()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' || ch == b',')
    {
        write!(target, "{}", decode_latin1(&kv.value))?;
    } else {
        write_script_string(target, &kv.value)?;
    }
    Ok(())
}
