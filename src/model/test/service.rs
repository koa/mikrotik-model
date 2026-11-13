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
            AppendFieldResult::UnknownField => {
                unknown.push(String::from_utf8_lossy(key).into_owned())
            }
            other => panic!(
                "{} = {}: {other:?}",
                String::from_utf8_lossy(key),
                String::from_utf8_lossy(value)
            ),
        }
    }
    (
        builder.build().expect("all required fields were supplied"),
        unknown,
    )
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
    assert_eq!(
        service.data.available_from.as_deref(),
        Some(b"192.168.0.0/16,10.1.2.3/32".as_slice())
    );
}

/// Before 7.24 the field is `address`: the entry still reads, without `available-from`.
#[test]
fn decodes_a_service_before_7_24() {
    let (service, _) = build(&[
        (b"name", b"api"),
        (b"address", b"10.0.0.0/8"),
        (b"disabled", b"false"),
    ]);
    assert_eq!(
        service.data.address.as_deref(),
        Some(b"10.0.0.0/8".as_slice())
    );
    assert_eq!(service.data.available_from, None);
}

/// The keyed print asks the whole menu and keeps the entries as the device's query did: its
/// API sessions are dynamic `api` entries next to the service (CHR 7.24.5), left out.
#[test]
fn a_keyed_print_keeps_the_static_entries_asked_for() {
    use crate::command::PrintKeyed;
    use crate::resource::KeyedRow;
    use mikrotik_api::prelude::{Command, ParsedMessage};
    let row =
        |fields: &[(&'static [u8], &'static [u8])]| -> Vec<(&'static [u8], Option<&'static [u8]>)> {
            fields
                .iter()
                .map(|(key, value)| (*key, Some(*value)))
                .collect()
        };
    let service = row(&[
        (b".id", b"*1"),
        (b"name", b"api"),
        (b"port", b"8728"),
        (b"dynamic", b"false"),
        (b"disabled", b"false"),
    ]);
    let session = row(&[
        (b".id", b"*8"),
        (b"name", b"api"),
        (b"remote", b"10.0.2.2:51234"),
        (b"dynamic", b"true"),
        (b"disabled", b"false"),
    ]);
    let www = row(&[
        (b".id", b"*2"),
        (b"name", b"www"),
        (b"port", b"80"),
        (b"dynamic", b"false"),
        (b"disabled", b"false"),
    ]);
    let kept = |command: &PrintKeyed<IpServiceByName>, row: &[(&[u8], Option<&[u8]>)]| {
        matches!(
            KeyedRow::parse_message(row, &command.context()),
            KeyedRow(Some(crate::resource::SentenceResult::Row { .. }))
        )
    };
    let all = PrintKeyed::<IpServiceByName>::all();
    assert!(kept(&all, &service));
    assert!(!kept(&all, &session));
    assert!(kept(&all, &www));
    let key = b"www".into();
    let one = PrintKeyed::<IpServiceByName>::with(&key);
    assert!(!kept(&one, &service));
    assert!(kept(&one, &www));
    // nothing but the print goes to the device
    let mut words = Vec::new();
    for word in mikrotik_api::prelude::Prepared::of(&one)
        .words()
        .split(|byte| *byte == 0)
    {
        words.push(String::from_utf8_lossy(word).into_owned());
    }
    assert!(!words.iter().any(|word| word.contains('?')), "{words:?}");
}

#[test]
fn entry_filters_judge_a_missing_field_as_the_device_does() {
    use crate::resource::EntryFilter;
    let row =
        |fields: &[(&'static [u8], &'static [u8])]| -> Vec<(&'static [u8], Option<&'static [u8]>)> {
            fields
                .iter()
                .map(|(key, value)| (*key, Some(*value)))
                .collect()
        };
    let bare = row(&[(b"name", b"x")]);
    assert!(!EntryFilter::NotBuiltin.keeps(&bare));
    assert!(!EntryFilter::NotDynamicNorBuiltin.keeps(&bare));
    assert!(EntryFilter::NotDynamic.keeps(&bare));
    let dynamic = row(&[(b"name", b"wifi1"), (b"dynamic", b"true")]);
    assert!(!EntryFilter::NotDynamic.keeps(&dynamic));
    let hardware = row(&[(b"default-name", b"wifi1"), (b"dynamic", b"true")]);
    assert!(EntryFilter::NotDynamicOrHardware(b"default-name").keeps(&hardware));
    assert!(!EntryFilter::NotDynamicOrHardware(b"default-name").keeps(&dynamic));
}
