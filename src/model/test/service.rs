//! Static decode tests for `/ip/service` (`ros_model/ip/Service.txt`), with the API output of
//! CHR 7.24.5.

use crate::model::{IpServiceByName, IpServiceByNameBuilder};
use crate::resource::{AppendFieldResult, DeserializeRosBuilder};

fn build(fields: &[(&[u8], &[u8])]) -> (IpServiceByName, Vec<String>) {
    let mut builder = IpServiceByNameBuilder::init(&Default::default());
    let mut unknown = Vec::new();
    for (key, value) in fields {
        match builder.append_field(key, Some(value)) {
            AppendFieldResult::Appended => {}
            AppendFieldResult::UnknownField => unknown.push(String::from_utf8_lossy(key).into_owned()),
            other => panic!("{} = {}: {other:?}", String::from_utf8_lossy(key), String::from_utf8_lossy(value)),
        }
    }
    (builder.build().expect("all required fields were supplied"), unknown)
}

/// 7.24.5 reports `available-from`, empty (read as none) when open to all: an open service
/// doesn't tell which of the two fields its version has.
#[test]
fn decodes_a_service_of_7_24() {
    let (service, _) = build(&[
        (b"name", b"api"),
        (b"port", b"8728"),
        (b"proto", b"tcp"),
        (b"available-from", b""),
        (b"vrf", b"main"),
        (b"max-sessions", b"20"),
        (b"disabled", b"false"),
    ]);
    assert_eq!(&*service.name, b"api".as_slice());
    assert_eq!(service.data.available_from, None);
    assert_eq!(service.data.address, None);
    let (service, _) = build(&[
        (b"name", b"telnet"),
        (b"available-from", b"192.168.0.0/16,10.1.2.3/32"),
        (b"disabled", b"true"),
    ]);
    assert!(service.data.disabled);
    assert_eq!(service.data.available_from.as_deref(), Some(b"192.168.0.0/16,10.1.2.3/32".as_slice()));
}

/// Before 7.24 the field is `address`: the entry still reads, without `available-from`.
#[test]
fn decodes_a_service_before_7_24() {
    let (service, _) = build(&[(b"name", b"api"), (b"address", b"10.0.0.0/8"), (b"disabled", b"false")]);
    assert_eq!(service.data.address.as_deref(), Some(b"10.0.0.0/8".as_slice()));
    assert_eq!(service.data.available_from, None);
}
