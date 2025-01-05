use crate::resource::{Error, TrapResponse};
use mikrotik_api::prelude::TrapCategory;

pub fn default_if_missing<V: Default>(result: Result<V, Error>) -> Result<V, Error> {
    match result {
        Ok(v) => Ok(v),
        Err(Error::Trap(TrapResponse {
            category: Some(TrapCategory::MissingItemOrCommand),
            message: _,
        })) => Ok(V::default()),
        Err(Error::Trap(TrapResponse {
            category: _,
            message,
        })) if message.as_ref() == b"no such command prefix" => Ok(V::default()),
        Err(e) => Err(e),
    }
}
