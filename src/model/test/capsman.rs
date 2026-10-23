//! `/caps-man/configuration` (`ros_model/caps-man/Configuration.txt`), checked on CHR 7.24.5
//! (its `wireless` package has the legacy CAPsMAN).
use crate::model::{CapsManConfigurationByName, CapsManConfigurationByNameBuilder};
use crate::resource::{CfgResource, DeserializeRosBuilder};

fn configuration(fields: &[(&[u8], &[u8])]) -> CapsManConfigurationByName {
    let mut builder = CapsManConfigurationByNameBuilder::init(&Default::default());
    for (key, value) in fields {
        builder.append_field(key, Some(value));
    }
    builder.build().unwrap()
}

/// A configuration losing its profiles (an open WLAN without its security profile): with two
/// profiles RouterOS refuses `security=""` ("ambiguous value of security, more than one
/// possible value matches input"), likewise `datapath`, `channel`, `rates`,
/// `datapath.bridge` and `datapath.interface-list`; `!field` takes each away.
#[test]
fn dropping_references_unsets_them() {
    let plain = configuration(&[(b".id", b"*1"), (b"name", b"open"), (b"ssid", b"open")]);
    let set = configuration(&[
        (b".id", b"*1"),
        (b"name", b"open"),
        (b"ssid", b"open"),
        (b"security", b"berg"),
        (b"datapath", b"berg"),
        (b"channel", b"berg"),
        (b"rates", b"berg"),
        (b"datapath.bridge", b"bridge"),
        (b"datapath.interface-list", b"lan"),
    ]);
    let mut fields = plain
        .0
        .changed_values(&set.0)
        .map(|field| {
            assert!(field.value.is_empty());
            String::from_utf8_lossy(field.key).into_owned()
        })
        .collect::<Vec<_>>();
    fields.sort();
    assert_eq!(
        fields,
        [
            "!channel",
            "!datapath",
            "!datapath.bridge",
            "!datapath.interface-list",
            "!rates",
            "!security",
        ]
    );
}
