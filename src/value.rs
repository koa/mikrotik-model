use log::warn;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;
use std::time::Duration;
use crate::model::YesNo::No;

pub enum ParseRosValueResult<V> {
    None,
    Value(V),
    Invalid,
}
impl<V> ParseRosValueResult<V> {
    pub fn map<R>(self, f: impl FnOnce(V) -> R) -> ParseRosValueResult<R> {
        match self {
            ParseRosValueResult::None => ParseRosValueResult::None,
            ParseRosValueResult::Value(v) => ParseRosValueResult::Value(f(v)),
            ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
        }
    }
}

impl<V> ParseRosValueResult<V> {
    pub fn ok(self) -> Option<V> {
        match self {
            ParseRosValueResult::None => None,
            ParseRosValueResult::Value(v) => Some(v),
            ParseRosValueResult::Invalid => None,
        }
    }
}

impl<V: Debug> Debug for ParseRosValueResult<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseRosValueResult::None => f.write_str("None"),
            ParseRosValueResult::Value(v) => {
                f.write_str("Value: ")?;
                v.fmt(f)
            }
            ParseRosValueResult::Invalid => f.write_str("Invalid"),
        }
    }
}
impl<V: PartialEq> PartialEq for ParseRosValueResult<V> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ParseRosValueResult::None, ParseRosValueResult::None) => true,
            (ParseRosValueResult::Value(lhs), ParseRosValueResult::Value(rhs)) => lhs == rhs,
            (ParseRosValueResult::Invalid, ParseRosValueResult::Invalid) => true,
            _ => false,
        }
    }
}
impl<V: Clone> Clone for ParseRosValueResult<V> {
    fn clone(&self) -> Self {
        match self {
            ParseRosValueResult::None => ParseRosValueResult::None,
            ParseRosValueResult::Value(v) => ParseRosValueResult::Value(v.clone()),
            ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
        }
    }
}
impl<V: Copy> Copy for ParseRosValueResult<V> {}

pub trait RosValue: Sized {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self>;
    fn encode_ros(&self) -> Cow<str>;
}

impl RosValue for Box<str> {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        ParseRosValueResult::Value(value.into())
    }

    fn encode_ros(&self) -> Cow<str> {
        self.as_ref().into()
    }
}

macro_rules! parameter_value_impl {
        ($($t:ty)*) => {$(
            impl RosValue for $t {
                fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
                    if value.is_empty() {
                        ParseRosValueResult::None
                    } else {
                        match <$t>::from_str(value) {
                            Ok(v) => ParseRosValueResult::Value(v),
                            Err(_) => ParseRosValueResult::Invalid,
                        }
                    }
                }

                fn encode_ros(&self) -> Cow<str> {
                    format!("{}", self).into()
                }
            }
        )*}
    }
parameter_value_impl! { isize i8 i16 i32 i64 i128 usize u8 u16 u32 u64 u128 f32 f64 bool}

impl RosValue for Duration {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        let mut ret = Duration::default();
        let mut chars = value.chars();
        let mut unit = None;
        let mut number = String::with_capacity(10);
        loop {
            #[allow(clippy::while_let_on_iterator)]
            while let Some(c) = chars.next() {
                if c.is_numeric() {
                    number.push(c);
                } else {
                    unit = Some(c);
                    break;
                }
            }
            if number.is_empty() && unit.is_none() {
                return ParseRosValueResult::Value(ret);
            }
            let count = match number.parse::<u64>() {
                Ok(v) => v,
                Err(e) => {
                    warn!("Cannot parse duration {}: {}", value, e);
                    return ParseRosValueResult::Invalid;
                }
            };
            let duration = match unit {
                None => {
                    return ParseRosValueResult::Value(ret);
                }
                Some('s') => Duration::from_secs( count),
                Some('m') => Duration::from_secs( 60 * count),
                Some('h') => Duration::from_secs( 3600 * count),
                Some('d') => Duration::from_secs(24 * 3600 * count),
                Some('w') => Duration::from_secs(7 * 24 * 3600 * count),
                Some(u) => {
                    warn!("Invalid time unit {u} on {value}");
                    return ParseRosValueResult::Invalid;
                }
            };
            number.clear();
            unit=None;
            ret += duration;
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        format!("{}s", self.as_secs()).into()
    }
}
