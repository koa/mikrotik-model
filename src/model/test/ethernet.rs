//! `/interface/ethernet` (`ros_model/interface/Ethernet.txt`).
use crate::hwconfig::{EthernetNamePattern, generate_ethernet};
use crate::model::{
    EthernetSpeed, InterfaceEthernetByDefaultName, InterfaceEthernetByDefaultNameBuilder,
};
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

#[test]
fn a_fixed_port_reports_its_speed() {
    let port = fixed();
    assert!(!port.data.auto_negotiation);
    assert_eq!(port.data.speed, Some(EthernetSpeed::_10GBaseCr));
    assert!(port.data.advertise.is_empty());
}

/// lab-crs226 `sfpplus2` (CRS226-24G-2S+, 7.24.6), its second cage after `sfp-sfpplus1`: the
/// port the pattern builds is the one the device has.
#[test]
fn an_sfpplus_cage_is_built_as_reported() {
    let mut builder = InterfaceEthernetByDefaultNameBuilder::init(&Default::default());
    for (key, value) in [
        (&b"default-name"[..], &b"sfpplus2"[..]),
        (b"name", b"sfpplus2"),
        (
            b"advertise",
            b"10M-baseT-half,10M-baseT-full,100M-baseT-half,100M-baseT-full,1G-baseT-half,\
              1G-baseT-full,1G-baseX,10G-baseT,10G-baseSR-LR,10G-baseCR",
        ),
        (b"arp", b"enabled"),
        (b"arp-timeout", b"auto"),
        (b"auto-negotiation", b"true"),
        (b"bandwidth", b"unlimited/unlimited"),
        (b"disabled", b"false"),
        (b"l2mtu", b"1588"),
        (b"loop-protect", b"default"),
        (b"loop-protect-disable-time", b"5m"),
        (b"loop-protect-send-interval", b"5s"),
        (b"mtu", b"1500"),
        (b"rx-flow-control", b"off"),
        (b"tx-flow-control", b"off"),
        (b"sfp-ignore-rx-los", b"false"),
        (b"sfp-rate-select", b"high"),
        (b"sfp-shutdown-temperature", b"95"),
    ] {
        builder.append_field(key, Some(value));
    }
    let reported = builder.build().unwrap();
    let speeds = [
        EthernetSpeed::_10MBaseTHalf,
        EthernetSpeed::_10MBaseTFull,
        EthernetSpeed::_100MBaseTHalf,
        EthernetSpeed::_100MBaseTFull,
        EthernetSpeed::_1GBaseTHalf,
        EthernetSpeed::_1GBaseTFull,
        EthernetSpeed::_1GBaseX,
        EthernetSpeed::_10GBaseT,
        EthernetSpeed::_10GBaseCr,
        EthernetSpeed::_10GBaseSrLr,
    ];
    let built = generate_ethernet(EthernetNamePattern::SfpPlus, &speeds, 1588, false)(2);
    assert_eq!(built.default_name, reported.default_name);
    let changed = built
        .data
        .changed_values(&reported.data)
        .map(|field| String::from_utf8_lossy(field.key).into_owned())
        .collect::<Vec<_>>();
    // the name is the provisioner's to give
    assert_eq!(changed, ["name"]);
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
