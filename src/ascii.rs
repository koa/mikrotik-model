use encoding_rs::mem::decode_latin1;
use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Write};
use std::ops::Deref;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct AsciiStringRef<'a>(pub &'a [u8]);

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
pub struct AsciiString(pub Box<[u8]>);

impl Debug for AsciiStringRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char('"')?;
        f.write_str(decode_latin1(self.0).as_ref())?;
        f.write_char('"')
    }
}
impl Display for AsciiStringRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(decode_latin1(self.0).as_ref())
    }
}

impl<'a> From<&'a [u8]> for AsciiStringRef<'a> {
    fn from(value: &'a [u8]) -> Self {
        AsciiStringRef(value)
    }
}

impl Debug for AsciiString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char('"')?;
        f.write_str(decode_latin1(&self.0).as_ref())?;
        f.write_char('"')
    }
}
impl Display for AsciiString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(decode_latin1(&self.0).as_ref())
    }
}

impl From<Box<[u8]>> for AsciiString {
    fn from(value: Box<[u8]>) -> Self {
        AsciiString(value)
    }
}
impl From<&[u8]> for AsciiString {
    fn from(value: &[u8]) -> Self {
        AsciiString(Box::from(value))
    }
}
impl<const N: usize> From<&[u8; N]> for AsciiString {
    fn from(value: &[u8; N]) -> Self {
        AsciiString(Box::from(value.as_slice()))
    }
}
impl From<String> for AsciiString {
    fn from(value: String) -> Self {
        AsciiString(Box::from(value.into_bytes()))
    }
}
impl From<&str> for AsciiString {
    fn from(value: &str) -> Self {
        AsciiString(Box::from(value.as_bytes()))
    }
}
impl Deref for AsciiString {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a> From<&'a AsciiString> for Cow<'a, str> {
    fn from(value: &'a AsciiString) -> Self {
        decode_latin1(&value.0)
    }
}
/*impl<I: IntoIterator<Item = u8>> From<I> for AsciiString {
    fn from(value: I) -> Self {
        AsciiString(value.into_iter().collect())
    }
}
impl<'a, I: IntoIterator<Item = &'a u8>> From<I> for AsciiString {
    fn from(value: I) -> Self {
        AsciiString(value.into_iter().copied().collect())
    }
}*/
