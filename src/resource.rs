use std::collections::HashMap;
use thiserror::Error;

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

pub trait RosResource: Sized {
    fn parse(values: &HashMap<String, Option<String>>) -> Result<Self, ResourceAccessError>;
    fn path() -> &'static str;
}
