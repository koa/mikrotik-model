use ipnet::IpNet;
use itertools::Itertools;
use log::{error, warn};
use mac_address::MacAddress;
use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter, Write};
use std::hash::Hash;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

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

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Hex<V: Copy + Eq + Hash>(V);

macro_rules! hex_value_impl {
        ($($t:ty)*) => {$(
            impl RosValue for  Hex<$t>  {
                fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
                    if value.is_empty() {
                        ParseRosValueResult::None
                    } else {
                        match if let Some(hex_value) = value.strip_prefix("0x") {
                            <$t>::from_str_radix(hex_value, 16)
                        } else {
                            value.parse::<$t>()
                        } {
                            Ok(v) => ParseRosValueResult::Value(Hex(v)),
                            Err(_) => ParseRosValueResult::Invalid,
                        }
                    }
                }

                fn encode_ros(&self) -> Cow<str> {
                    format!("0x{:X}", self.0).into()
                }
            }
            impl Debug for Hex<$t>{
                fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                    write!(f, "0x{:X}", self.0)
                }
            }
        )*}
    }
hex_value_impl! { isize i8 i16 i32 i64 i128 usize u8 u16 u32 u64 u128}

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
                Some('s') => Duration::from_secs(count),
                Some('m') => Duration::from_secs(60 * count),
                Some('h') => Duration::from_secs(3600 * count),
                Some('d') => Duration::from_secs(24 * 3600 * count),
                Some('w') => Duration::from_secs(7 * 24 * 3600 * count),
                Some(u) => {
                    warn!("Invalid time unit {u} on {value}");
                    return ParseRosValueResult::Invalid;
                }
            };
            number.clear();
            unit = None;
            ret += duration;
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        format!("{}s", self.as_secs()).into()
    }
}
impl RosValue for MacAddress {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        match MacAddress::from_str(value) {
            Ok(v) => ParseRosValueResult::Value(v),
            Err(_) => ParseRosValueResult::Invalid,
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        self.to_string().into()
    }
}
impl<V: RosValue> RosValue for Option<V> {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if value.is_empty() {
            ParseRosValueResult::None
        } else {
            match V::parse_ros(value) {
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(Some(v)),
                ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        match self {
            None => "".into(),
            Some(v) => v.encode_ros(),
        }
    }
}
impl<V: RosValue + Hash + Eq> RosValue for HashSet<V> {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if value.is_empty() {
            ParseRosValueResult::Value(HashSet::new())
        } else {
            let mut result = HashSet::new();
            for value in value.split(',').map(V::parse_ros) {
                match value {
                    ParseRosValueResult::None => {}
                    ParseRosValueResult::Value(v) => {
                        result.insert(v);
                    }
                    ParseRosValueResult::Invalid => return ParseRosValueResult::Invalid,
                }
            }
            ParseRosValueResult::Value(result)
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        let string = self.iter().map(|v| v.encode_ros()).join(",");
        string.into()
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Auto<V: RosValue> {
    Auto,
    Value(V),
}
impl<V: RosValue> RosValue for Auto<V> {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if value == "auto" {
            ParseRosValueResult::Value(Auto::Auto)
        } else {
            match RosValue::parse_ros(value) {
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(Auto::Value(v)),
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        match self {
            Auto::Auto => "auto".into(),
            Auto::Value(v) => v.encode_ros(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct RxTxPair<V: RosValue> {
    pub rx: V,
    pub tx: V,
}
impl<V: RosValue> RosValue for RxTxPair<V> {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if value.is_empty() {
            ParseRosValueResult::None
        } else if let Some((rx_value, tx_value)) = value.split_once('/') {
            match (
                <V as RosValue>::parse_ros(rx_value),
                <V as RosValue>::parse_ros(tx_value),
            ) {
                (ParseRosValueResult::Value(rx), ParseRosValueResult::Value(tx)) => {
                    ParseRosValueResult::Value(RxTxPair { rx, tx })
                }
                (ParseRosValueResult::Invalid, _) | (_, ParseRosValueResult::Invalid) => {
                    ParseRosValueResult::Invalid
                }
                _ => ParseRosValueResult::None,
            }
        } else {
            ParseRosValueResult::Invalid
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        format!("{}/{}", self.tx.encode_ros(), self.rx.encode_ros()).into()
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum HasNone<V: RosValue> {
    NoneValue,
    Value(V),
}
impl<V: RosValue> RosValue for HasNone<V> {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if value == "none" {
            ParseRosValueResult::Value(HasNone::NoneValue)
        } else {
            match RosValue::parse_ros(value) {
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(HasNone::Value(v)),
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        match self {
            HasNone::NoneValue => "none".into(),
            HasNone::Value(v) => v.encode_ros(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum HasUnlimited<V: RosValue> {
    Unlimited,
    Value(V),
}
impl<V: RosValue> RosValue for HasUnlimited<V> {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if value == "unlimited" {
            ParseRosValueResult::Value(HasUnlimited::Unlimited)
        } else {
            match RosValue::parse_ros(value) {
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(HasUnlimited::Value(v)),
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        match self {
            HasUnlimited::Unlimited => "unlimited".into(),
            HasUnlimited::Value(v) => v.encode_ros(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum HasDisabled<V: RosValue> {
    Disabled,
    Value(V),
}
impl<V: RosValue> RosValue for HasDisabled<V> {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if value == "disabled" {
            ParseRosValueResult::Value(HasDisabled::Disabled)
        } else {
            match RosValue::parse_ros(value) {
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(HasDisabled::Value(v)),
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        match self {
            HasDisabled::Disabled => "disabled".into(),
            HasDisabled::Value(v) => v.encode_ros(),
        }
    }
}

impl RosValue for IpAddr {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if value.is_empty() {
            ParseRosValueResult::None
        } else {
            match value.parse::<IpAddr>() {
                Ok(v) => ParseRosValueResult::Value(v),
                Err(_) => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        self.to_string().into()
    }
}

impl RosValue for IpNet {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if value.is_empty() {
            ParseRosValueResult::None
        } else {
            match IpNet::from_str(value) {
                Ok(v) => ParseRosValueResult::Value(v),
                Err(_) => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        format!("{}", self).into()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct IpWithInterface {
    pub ip: IpAddr,
    pub interface: Box<str>,
}
impl RosValue for IpWithInterface {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if let Some((ip, if_name)) = value.split_once('%') {
            IpAddr::parse_ros(ip).map(|ip| IpWithInterface {
                ip,
                interface: if_name.into(),
            })
        } else {
            ParseRosValueResult::Invalid
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        format!("{}%{}", self.ip.encode_ros(), self.interface).into()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum IpOrInterface {
    Ip(IpAddr),
    Interface(Box<str>),
    IpWithInterface(IpWithInterface),
}

impl From<IpAddr> for IpOrInterface {
    fn from(ip: IpAddr) -> Self {
        IpOrInterface::Ip(ip)
    }
}

impl From<IpWithInterface> for IpOrInterface {
    fn from(ip: IpWithInterface) -> Self {
        IpOrInterface::IpWithInterface(ip)
    }
}

impl RosValue for IpOrInterface {
    fn parse_ros(value: &str) -> ParseRosValueResult<Self> {
        if value.contains('%') {
            IpWithInterface::parse_ros(value).map(IpOrInterface::IpWithInterface)
        } else {
            match IpAddr::parse_ros(value).map(IpOrInterface::Ip) {
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(v),
                ParseRosValueResult::Invalid => Box::parse_ros(value).map(IpOrInterface::Interface),
            }
        }
    }

    fn encode_ros(&self) -> Cow<str> {
        match self {
            IpOrInterface::Ip(ip) => ip.encode_ros(),
            IpOrInterface::Interface(ifname) => ifname.encode_ros(),
            IpOrInterface::IpWithInterface(ip_net) => ip_net.encode_ros(),
        }
    }
}

pub struct ModifiedValue<'a> {
    pub key: &'static str,
    pub value: Cow<'a, str>,
}

pub fn write_script_string(target: &mut impl Write, value: &str) -> core::fmt::Result {
    target.write_char('"')?;
    for character in value.chars() {
        match character {
            '0'..='9' | 'A'..='Z' | 'a'..='z' | ' ' => target.write_char(character)?,
            '"' | '\\' => {
                target.write_char('\\')?;
                target.write_char(character)?;
            }
            '\n' => target.write_str("\\n")?,
            '\r' => target.write_str("\\r")?,
            '\t' => target.write_str("\\t")?,
            '\x07' => target.write_str("\\a")?,
            '\x08' => target.write_str("\\b")?,
            ch => {
                if (ch as u32) < 256 {
                    target.write_char('\\')?;
                    write!(target, "{:X}", ch as u8)?;
                } else {
                    error!("Skipping invalid character in string {value}: {character}")
                }
            }
        }
    }
    target.write_char('"')?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_option_parse() {
        let x: ParseRosValueResult<Option<Box<str>>> = RosValue::parse_ros("");
        println!("x: {x:?}");
    }
    #[test]
    fn test_hex_parse() {
        let parsed: ParseRosValueResult<Hex<u16>> = RosValue::parse_ros("0x8000");
        assert_eq!(parsed, ParseRosValueResult::Value(Hex(0x8000)));
        let encoded = Hex(0x8000).encode_ros();
        assert_eq!(encoded, "0x8000");
    }
}
