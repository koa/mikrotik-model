use encoding_rs::mem::decode_latin1;
use std::fmt::{Debug, Display, Formatter, Write};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct AsciiStringRef<'a>(pub &'a [u8]);

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
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
