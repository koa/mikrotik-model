//! `/interface/ethernet` (`ros_model/interface/Ethernet.txt`).
use crate::model::{InterfaceEthernetByDefaultName, InterfaceEthernetByDefaultNameBuilder};
use crate::resource::{CfgResource, DeserializeRosBuilder};

/// klr-sw-1 `qsfpplus1-1` as RouterOS 7 reports it with a fixed speed (no `advertise`).
fn fixed() -> InterfaceEthernetByDefaultName {
    let mut builder = InterfaceEthernetByDefaultNameBuilder::init(&Default::default());
    for (key, value) in [
        (&b"default-name"[..], &b"qsfpplus1-1"[..]),
        (b"name", b"q01-1-klr-sw02"),
        (b"mtu", b"1500"),
        (b"l2mtu", b"1592"),
        (b"auto-negotiation", b"false"),
        (b"speed", b"10G-baseCR"),
        (b"loop-protect", b"default"),
        (b"disabled", b"false"),
        (b"arp", b"enabled"),
        (b"loop-protect-disable-time", b"5m"),
        (b"loop-protect-send-interval", b"5s"),
    ] {
        builder.append_field(key, Some(value));
    }
    builder.build().unwrap()
}

/// Negotiating again: RouterOS refuses `speed=""` ("ambiguous value of speed"), and a port
/// that negotiates doesn't use it, so a target without one keeps it.
#[test]
fn negotiating_again_keeps_the_speed() {
    let current = fixed();
    let mut target = current.data.clone();
    target.auto_negotiation = true;
    target.speed = None;
    let fields = target
        .changed_values(&current.data)
        .map(|field| String::from_utf8_lossy(field.key).into_owned())
        .collect::<Vec<_>>();
    assert_eq!(fields, ["auto-negotiation"]);
}
