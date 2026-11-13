//! The value sets several menus share (`enums.yaml`), with the values the menus' own `enum=`
//! lacked: `/console/inspect` completions of lab-sxt-a, lab-np2 (7.24.6) and a CHR 7.24.5.

use crate::model::{Arp, PoeOut, PoeVoltage, WirelessHwProtectionMode, WirelessMulticastHelper};
use crate::value::{Auto, HasDisabled, HasNone, ParseRosValueResult, RosValue};

fn parsed<V: RosValue>(value: &[u8]) -> V {
    match V::parse_ros(value) {
        ParseRosValueResult::Value(value) => value,
        _ => panic!("{} not accepted", String::from_utf8_lossy(value)),
    }
}

#[test]
fn every_interface_takes_arp_disabled() {
    // the wifi and CAPsMAN menus had no `disabled`
    assert_eq!(parsed::<Arp>(b"disabled"), Arp::Disabled);
    assert_eq!(Arp::Disabled.encode_ros().as_ref(), b"disabled");
}

#[test]
fn the_legacy_wireless_helps_multicast_by_dhcp() {
    assert_eq!(
        parsed::<HasDisabled<WirelessMulticastHelper>>(b"dhcp"),
        HasDisabled::Value(WirelessMulticastHelper::Dhcp)
    );
    assert_eq!(
        parsed::<HasDisabled<WirelessMulticastHelper>>(b"disabled"),
        HasDisabled::Disabled
    );
}

#[test]
fn no_hw_protection_is_none() {
    assert_eq!(
        parsed::<HasNone<WirelessHwProtectionMode>>(b"none"),
        HasNone::NoneValue
    );
    assert_eq!(
        parsed::<HasNone<WirelessHwProtectionMode>>(b"rts-cts"),
        HasNone::Value(WirelessHwProtectionMode::RtsCts)
    );
}

#[test]
fn an_ethernet_port_feeds_poe_by_pair() {
    assert_eq!(parsed::<PoeOut>(b"forced-on-a"), PoeOut::ForcedOnA);
    assert_eq!(parsed::<PoeOut>(b"forced-on-bt"), PoeOut::ForcedOnBt);
    assert_eq!(parsed::<Auto<PoeVoltage>>(b"auto"), Auto::Auto);
    assert_eq!(
        parsed::<Auto<PoeVoltage>>(b"high"),
        Auto::Value(PoeVoltage::High)
    );
}
