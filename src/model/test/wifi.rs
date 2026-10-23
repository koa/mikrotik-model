//! References of `/interface/wifi` and `/interface/wifi/configuration`
//! (`ros_model/interface/Wifi.txt`), checked on CHR 7.24.5.
use crate::model::{
    InterfaceWifiByDefaultName, InterfaceWifiByDefaultNameBuilder,
    InterfaceWifiConfigurationByName, InterfaceWifiConfigurationByNameBuilder,
};
use crate::resource::{CfgResource, DeserializeRosBuilder, DeserializeRosResource};

fn build<B, R>(fields: &[(&[u8], &[u8])]) -> R
where
    B: DeserializeRosBuilder<R>,
    B::Context: Default,
    R: DeserializeRosResource,
{
    let mut builder = B::init(&B::Context::default());
    for (key, value) in fields {
        builder.append_field(key, Some(value));
    }
    builder.build().unwrap()
}

/// `fields` and `more`.
fn with<'a>(fields: &[(&'a [u8], &'a [u8])], more: &[(&'a [u8], &'a [u8])]) -> Vec<(&'a [u8], &'a [u8])> {
    [fields, more].concat()
}

/// The keys `changed_values` writes, sorted; none with a value.
fn unset_keys<'a>(fields: impl Iterator<Item = crate::value::KeyValuePair<'a>>) -> Vec<String> {
    let mut keys = fields
        .map(|field| {
            assert!(field.value.is_empty(), "{}", String::from_utf8_lossy(field.key));
            String::from_utf8_lossy(field.key).into_owned()
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

/// A configuration losing its profiles: with two of a kind RouterOS refuses `security=""`
/// ("ambiguous value of security, more than one possible value matches input"), likewise
/// `channel`, `aaa`, `interworking`, `steering` and `datapath.interface-list`; `!field` takes
/// each away.
#[test]
fn a_configuration_dropping_references_unsets_them() {
    let fields: &[(&[u8], &[u8])] = &[(b".id", b"*1"), (b"name", b"open"), (b"ssid", b"open")];
    let plain: InterfaceWifiConfigurationByName =
        build::<InterfaceWifiConfigurationByNameBuilder, _>(fields);
    let set: InterfaceWifiConfigurationByName = build::<InterfaceWifiConfigurationByNameBuilder, _>(
        &with(fields, &[
                (b"security", b"berg"),
                (b"channel", b"berg"),
                (b"aaa", b"berg"),
                (b"interworking", b"berg"),
                (b"steering", b"berg"),
                (b"datapath.interface-list", b"lan"),
        ]),
    );
    assert_eq!(
        unset_keys(plain.0.changed_values(&set.0)),
        [
            "!aaa",
            "!channel",
            "!datapath.interface-list",
            "!interworking",
            "!security",
            "!steering",
        ]
    );
}

/// A radio losing its security profile (an open WLAN) or configuration the same way, and
/// values set on it by hand: RouterOS refuses `security.connect-priority=""` ("invalid value
/// for argument"), as every number, flag, duration and choice of these groups (checked on the
/// configuration, which has the same fields); `!field` takes each away.
#[test]
fn a_radio_dropping_references_unsets_them() {
    let fields: &[(&[u8], &[u8])] =
        &[(b".id", b"*1"), (b"default-name", b"wifi1"), (b"name", b"wifi1"), (b"disabled", b"false")];
    let plain: InterfaceWifiByDefaultName = build::<InterfaceWifiByDefaultNameBuilder, _>(fields);
    let set: InterfaceWifiByDefaultName = build::<InterfaceWifiByDefaultNameBuilder, _>(
        &with(fields, &[
                (b"security", b"berg"),
                (b"configuration", b"berg"),
                (b"datapath.interface-list", b"lan"),
                (b"security.connect-priority", b"5"),
                (b"security.ft", b"true"),
                (b"channel.width", b"20mhz"),
                (b"configuration.tx-power", b"10"),
        ]),
    );
    assert_eq!(
        unset_keys(plain.data.changed_values(&set.data)),
        [
            "!channel.width",
            "!configuration",
            "!configuration.tx-power",
            "!datapath.interface-list",
            "!security",
            "!security.connect-priority",
            "!security.ft",
        ]
    );
}
