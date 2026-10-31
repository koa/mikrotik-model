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

/// A PoE port no longer watching its device (lab-np2, CRS318, 7.23.8): RouterOS refuses
/// `power-cycle-ping-address=""` ("invalid value for argument ip"), `!field` takes the address
/// and the timeout away (`!` checked on CHR 7.24.5, which has no PoE).
#[test]
fn dropping_the_power_cycle_ping_unsets_it() {
    let mut builder = InterfaceEthernetByDefaultNameBuilder::init(&Default::default());
    for (key, value) in [
        (&b"default-name"[..], &b"ether1"[..]),
        (b"name", b"e01-sxt-b"),
        (b"mtu", b"1500"),
        (b"l2mtu", b"1592"),
        (b"loop-protect", b"default"),
        (b"disabled", b"false"),
        (b"arp", b"enabled"),
        (b"loop-protect-disable-time", b"5m"),
        (b"loop-protect-send-interval", b"5s"),
        (b"power-cycle-ping-enabled", b"true"),
        (b"power-cycle-ping-address", b"192.168.101.12"),
        (b"power-cycle-ping-timeout", b"5m"),
    ] {
        builder.append_field(key, Some(value));
    }
    let current = builder.build().unwrap();
    let mut target = current.data.clone();
    target.power_cycle_ping_enabled = Some(false);
    target.power_cycle_ping_address = None;
    target.power_cycle_ping_timeout = None;
    let mut fields = target
        .changed_values(&current.data)
        .map(|field| {
            let value = String::from_utf8_lossy(&field.value).into_owned();
            format!("{}={value}", String::from_utf8_lossy(field.key))
        })
        .collect::<Vec<_>>();
    fields.sort();
    assert_eq!(
        fields,
        [
            "!power-cycle-ping-address=",
            "!power-cycle-ping-timeout=",
            "power-cycle-ping-enabled=no",
        ]
    );
}
