//! What a mutation says was there before (`ResourceMutation::before`), on
//! `/ip/dhcp-server/network`; not the mutations themselves.
use crate::model::{IpDhcpServerNetworkByAddress, IpDhcpServerNetworkByAddressBuilder};
use crate::resource::{
    DeserializeRosBuilder, KeyedResource, Updatable, removed_values, values_before,
};
use crate::value::KeyValuePair;
use std::borrow::Cow;

fn network(fields: &[(&[u8], &[u8])]) -> IpDhcpServerNetworkByAddress {
    let mut builder = IpDhcpServerNetworkByAddressBuilder::init(&Default::default());
    for (key, value) in fields {
        builder.append_field(key, Some(value));
    }
    builder.build().unwrap()
}

fn pairs(fields: &[KeyValuePair]) -> Vec<String> {
    fields
        .iter()
        .map(|field| {
            format!(
                "{}={}",
                String::from_utf8_lossy(field.key),
                String::from_utf8_lossy(&field.value)
            )
        })
        .collect()
}

/// An update names the old values of the fields it sets, also of one it takes away with
/// `!field`, under the field's own name.
#[test]
fn an_update_names_the_old_values() {
    let set = network(&[
        (b".id", b"*1"),
        (b"address", b"10.192.7.0/24"),
        (b"gateway", b"10.192.7.1"),
        (b"next-server", b"10.192.7.5"),
    ]);
    let target = network(&[
        (b".id", b"*1"),
        (b"address", b"10.192.7.0/24"),
        (b"gateway", b"10.192.7.254"),
    ]);
    let update = target.calculate_update(&set);
    assert_eq!(
        pairs(&update.fields),
        ["gateway=10.192.7.254", "!next-server="]
    );
    assert_eq!(
        pairs(&update.before),
        ["gateway=10.192.7.1", "next-server=10.192.7.5"]
    );
}

/// A removal names every field the entry had a value in.
#[test]
fn a_removal_names_the_entry() {
    let set = network(&[
        (b".id", b"*1"),
        (b"address", b"10.192.7.0/24"),
        (b"gateway", b"10.192.7.1"),
    ]);
    let before = pairs(&removed_values(&set));
    assert!(before.contains(&"address=10.192.7.0/24".to_string()));
    assert!(before.contains(&"gateway=10.192.7.1".to_string()));
    assert_eq!(set.key_value().to_string(), "10.192.7.0/24");
}

/// A field the comparison the other way doesn't name has no old value.
#[test]
fn a_field_without_old_value_is_left_out() {
    let fields = [
        KeyValuePair {
            key: b"a",
            value: Cow::Borrowed(b"1"),
        },
        KeyValuePair {
            key: b"b",
            value: Cow::Borrowed(b"2"),
        },
    ];
    let reverse = [KeyValuePair {
        key: b"b",
        value: Cow::Borrowed(b"0"),
    }];
    assert_eq!(pairs(&values_before(&fields, reverse.into_iter())), ["b=0"]);
}
