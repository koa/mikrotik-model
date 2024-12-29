use encoding_rs::mem::{decode_latin1, encode_latin1_lossy};
use ipnet::IpNet;
use log::warn;
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
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self>;
    fn encode_ros(&self) -> Cow<[u8]>;
}

impl RosValue for Box<[u8]> {
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        ParseRosValueResult::Value(value.into())
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        self.as_ref().into()
    }
}

macro_rules! parameter_value_impl {
        ($($t:ty)*) => {$(
            impl RosValue for $t {
                fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
                    if value.is_empty() {
                        ParseRosValueResult::None
                    } else if !value.is_ascii(){
                        ParseRosValueResult::Invalid
                    } else {
                        match <$t>::from_str(String::from_utf8_lossy(value).as_ref()) {
                            Ok(v) => ParseRosValueResult::Value(v),
                            Err(_) => ParseRosValueResult::Invalid,
                        }
                    }
                }

                fn encode_ros(&self) -> Cow<[u8]> {
                    Cow::Owned(self.to_string().as_bytes().into())

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
                fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
                    if value.is_empty() {
                        ParseRosValueResult::None
                    } else if !value.is_ascii(){
                        ParseRosValueResult::Invalid
                    } else {
                        match if let Some(hex_value) = value.strip_prefix(b"0x") {
                            <$t>::from_str_radix(String::from_utf8_lossy(hex_value).as_ref(), 16)
                        } else {
                            String::from_utf8_lossy(value).parse::<$t>()
                        } {
                            Ok(v) => ParseRosValueResult::Value(Hex(v)),
                            Err(_) => ParseRosValueResult::Invalid,
                        }
                    }
                }

                fn encode_ros(&self) -> Cow<[u8]> {
                    Cow::Owned( Vec::from( format!("0x{:X}", self.0).as_bytes()))
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
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        let mut ret = Duration::default();
        let mut chars = value.iter();
        let mut unit = None;
        let mut number = 0;
        loop {
            #[allow(clippy::while_let_on_iterator)]
            while let Some(c) = chars.next() {
                if c.is_ascii_digit() {
                    number = number * 10 + (c - b'0') as u64;
                } else {
                    unit = Some(c);
                    break;
                }
            }
            if number == 0 && unit.is_none() {
                return ParseRosValueResult::Value(ret);
            }
            let duration = match unit {
                None => {
                    return ParseRosValueResult::Value(ret);
                }
                Some(b's') => Duration::from_secs(number),
                Some(b'm') => Duration::from_secs(60 * number),
                Some(b'h') => Duration::from_secs(3600 * number),
                Some(b'd') => Duration::from_secs(24 * 3600 * number),
                Some(b'w') => Duration::from_secs(7 * 24 * 3600 * number),
                Some(&u) => {
                    warn!("Invalid time unit {} on {value:?}", u as char);
                    return ParseRosValueResult::Invalid;
                }
            };
            number = 0;
            unit = None;
            ret += duration;
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        Cow::Owned(Vec::from(format!("{}s", self.as_secs()).as_bytes()))
    }
}
impl RosValue for MacAddress {
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        match MacAddress::from_str(String::from_utf8_lossy(value).as_ref()) {
            Ok(v) => ParseRosValueResult::Value(v),
            Err(_) => ParseRosValueResult::Invalid,
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        self.to_string().into_bytes().into()
    }
}
impl<V: RosValue> RosValue for Option<V> {
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
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

    fn encode_ros(&self) -> Cow<[u8]> {
        match self {
            None => b"".into(),
            Some(v) => v.encode_ros(),
        }
    }
}
impl<V: RosValue + Hash + Eq> RosValue for HashSet<V> {
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        if value.is_empty() {
            ParseRosValueResult::Value(HashSet::new())
        } else {
            let mut result = HashSet::new();
            for value in value.split(|ch| *ch == b',').map(V::parse_ros) {
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

    fn encode_ros(&self) -> Cow<[u8]> {
        let mut ret = Vec::new();
        for value in self {
            ret.extend_from_slice(value.encode_ros().as_ref());
        }
        ret.into()
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Auto<V: RosValue> {
    Auto,
    Value(V),
}
impl<V: RosValue> RosValue for Auto<V> {
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        if value == b"auto" {
            ParseRosValueResult::Value(Auto::Auto)
        } else {
            match RosValue::parse_ros(value) {
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(Auto::Value(v)),
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        match self {
            Auto::Auto => b"auto".into(),
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
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        if value.is_empty() {
            ParseRosValueResult::None
        } else {
            let mut values = value.splitn(2, |ch| *ch == b'/').map(V::parse_ros);
            let rx = values.next();
            let tx = values.next();
            match (rx, tx) {
                (Some(ParseRosValueResult::Value(rx)), Some(ParseRosValueResult::Value(tx))) => {
                    ParseRosValueResult::Value(RxTxPair { rx, tx })
                }
                (None, _)
                | (_, None)
                | (Some(ParseRosValueResult::Invalid), _)
                | (_, Some(ParseRosValueResult::Invalid)) => ParseRosValueResult::Invalid,
                _ => ParseRosValueResult::None,
            }
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        [
            self.rx.encode_ros().as_ref(),
            b"/",
            self.tx.encode_ros().as_ref(),
        ]
        .concat()
        .into()
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum HasNone<V: RosValue> {
    NoneValue,
    Value(V),
}
impl<V: RosValue> RosValue for HasNone<V> {
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        if value == b"none" {
            ParseRosValueResult::Value(HasNone::NoneValue)
        } else {
            match RosValue::parse_ros(value) {
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(HasNone::Value(v)),
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        match self {
            HasNone::NoneValue => b"none".into(),
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
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        if value == b"unlimited" {
            ParseRosValueResult::Value(HasUnlimited::Unlimited)
        } else {
            match RosValue::parse_ros(value) {
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(HasUnlimited::Value(v)),
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        match self {
            HasUnlimited::Unlimited => b"unlimited".into(),
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
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        if value == b"disabled" {
            ParseRosValueResult::Value(HasDisabled::Disabled)
        } else {
            match RosValue::parse_ros(value) {
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(HasDisabled::Value(v)),
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Invalid => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        match self {
            HasDisabled::Disabled => b"disabled".into(),
            HasDisabled::Value(v) => v.encode_ros(),
        }
    }
}

impl RosValue for IpAddr {
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        if value.is_empty() {
            ParseRosValueResult::None
        } else {
            match decode_latin1(value).parse::<IpAddr>() {
                Ok(v) => ParseRosValueResult::Value(v),
                Err(_) => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        Vec::from(encode_latin1_lossy(&self.to_string())).into()
    }
}

impl RosValue for IpNet {
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        if value.is_empty() {
            ParseRosValueResult::None
        } else {
            match IpNet::from_str(decode_latin1(value).as_ref()) {
                Ok(v) => ParseRosValueResult::Value(v),
                Err(_) => ParseRosValueResult::Invalid,
            }
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        Vec::from(encode_latin1_lossy(&format!("{}", self))).into()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct IpWithInterface {
    pub ip: IpAddr,
    pub interface: Box<[u8]>,
}
impl RosValue for IpWithInterface {
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        if let Some((ip, if_name)) = split_once(value, b'%') {
            IpAddr::parse_ros(ip).map(|ip| IpWithInterface {
                ip,
                interface: if_name.into(),
            })
        } else {
            ParseRosValueResult::Invalid
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        [
            self.ip.encode_ros().as_ref(),
            b"%",
            self.interface.encode_ros().as_ref(),
        ]
        .concat()
        .into()
    }
}

fn split_once(value: &[u8], char: u8) -> Option<(&[u8], &[u8])> {
    let mut parts = value.splitn(2, |&ch| ch == char);
    let first_part = parts.next();
    let second_part = parts.next();
    Option::zip(first_part, second_part)
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum IpOrInterface {
    Ip(IpAddr),
    Interface(Box<[u8]>),
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
    fn parse_ros(value: &[u8]) -> ParseRosValueResult<Self> {
        if value.contains(&b'%') {
            IpWithInterface::parse_ros(value).map(IpOrInterface::IpWithInterface)
        } else {
            match IpAddr::parse_ros(value).map(IpOrInterface::Ip) {
                ParseRosValueResult::None => ParseRosValueResult::None,
                ParseRosValueResult::Value(v) => ParseRosValueResult::Value(v),
                ParseRosValueResult::Invalid => Box::parse_ros(value).map(IpOrInterface::Interface),
            }
        }
    }

    fn encode_ros(&self) -> Cow<[u8]> {
        match self {
            IpOrInterface::Ip(ip) => ip.encode_ros(),
            IpOrInterface::Interface(ifname) => ifname.encode_ros(),
            IpOrInterface::IpWithInterface(ip_net) => ip_net.encode_ros(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct KeyValuePair<'a> {
    pub key: &'static [u8],
    pub value: Cow<'a, [u8]>,
}

pub fn write_script_string(target: &mut impl Write, value: &[u8]) -> core::fmt::Result {
    target.write_char('"')?;
    for character in value.iter().copied() {
        match character {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b' ' | b'.' | b'-' => {
                target.write_char(character as char)?
            }
            b'"' | b'\\' => {
                target.write_char('\\')?;
                target.write_char(character as char)?;
            }
            b'\n' => target.write_str("\\n")?,
            b'\r' => target.write_str("\\r")?,
            b'\t' => target.write_str("\\t")?,
            b'\x07' => target.write_str("\\a")?,
            b'\x08' => target.write_str("\\b")?,
            ch => {
                target.write_char('\\')?;
                write!(target, "{:X}", ch as u8)?;
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
        let x: ParseRosValueResult<Option<Box<[u8]>>> = RosValue::parse_ros(b"");
        println!("x: {x:?}");
    }
    #[test]
    fn test_hex_parse() {
        let parsed: ParseRosValueResult<Hex<u16>> = RosValue::parse_ros(b"0x8000");
        assert_eq!(parsed, ParseRosValueResult::Value(Hex(0x8000)));
        let encoded = Hex(0x8000).encode_ros();
        assert_eq!(encoded.as_ref(), b"0x8000");
    }
}
