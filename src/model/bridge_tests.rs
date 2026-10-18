//! Static decode tests for `/interface/bridge` (`ros_model/bridge.txt`).
//!
//! RouterOS only reports `mvrp` on a bridge with `vlan-filtering=yes` (checked on CHR
//! 7.24.5 and seen on `ap-klr-solar`, 7.23.7), so the field must stay optional, or
//! fetching a device with a plain bridge fails.

use crate::model::{InterfaceBridgeByName, InterfaceBridgeByNameBuilder};
use crate::resource::{DeserializeRosBuilder, DeserializeRosResource};

fn build<B, R>(fields: &[(&[u8], &[u8])]) -> R
where
    B: DeserializeRosBuilder<R> + Default,
    B::Context: Default,
    R: DeserializeRosResource,
{
    let mut builder = B::init(&B::Context::default());
    for (key, value) in fields {
        let result = builder.append_field(key, Some(value));
        assert!(
            matches!(result, crate::resource::AppendFieldResult::Appended),
            "field {} = {} was not accepted: {:?}",
            String::from_utf8_lossy(key),
            String::from_utf8_lossy(value),
            result
        );
    }
    builder.build().expect("all required fields were supplied")
}

/// The required configuration fields of a bridge, without any MVRP field.
const BRIDGE_WITHOUT_MVRP: &[(&[u8], &[u8])] = &[
    (b"name", b"bridge"),
    (b"ageing-time", b"5m"),
    (b"arp", b"enabled"),
    (b"arp-timeout", b"auto"),
    (b"auto-mac", b"true"),
    (b"dhcp-snooping", b"false"),
    (b"disabled", b"false"),
    (b"fast-forward", b"true"),
    (b"igmp-snooping", b"false"),
    (b"max-learned-entries", b"auto"),
    (b"mtu", b"auto"),
    (b"port-cost-mode", b"long"),
    (b"protocol-mode", b"rstp"),
    (b"vlan-filtering", b"false"),
];

#[test]
fn test_bridge_without_mvrp() {
    let bridge: InterfaceBridgeByName =
        build::<InterfaceBridgeByNameBuilder, _>(BRIDGE_WITHOUT_MVRP);
    assert_eq!(bridge.0.mvrp, None);
}

#[test]
fn test_bridge_with_mvrp() {
    let mut fields = BRIDGE_WITHOUT_MVRP.to_vec();
    fields.push((b"mvrp", b"false"));
    let bridge: InterfaceBridgeByName = build::<InterfaceBridgeByNameBuilder, _>(&fields);
    assert_eq!(bridge.0.mvrp, Some(false));
}

/// A port of srv-router-berg (RouterOS 7.23.7) reports `multicast-router=false`, which
/// RouterOS doesn't take on `set` ("input does not match any value", CHR 7.24.5): it must
/// still parse, or the whole device can't be read.
#[test]
fn test_port_multicast_router_false() {
    use crate::model::InterfaceBridgePortMulticastRouter;
    use crate::value::{ParseRosValueResult, RosValue};
    assert!(matches!(
        InterfaceBridgePortMulticastRouter::parse_ros(b"false"),
        ParseRosValueResult::Value(InterfaceBridgePortMulticastRouter::False)
    ));
}

/// Two ports trading their bridges (router-stall-burri, 7.23.8): RouterOS has an interface in
/// one port only ("device already added as bridge port"), so each port keeps its interface
/// and changes its bridge; setting one to the other's interface failed.
#[test]
fn ports_trading_bridges_keep_their_interfaces() {
    use crate::{
        model::{InterfaceBridgePortById, InterfaceBridgePortCfg},
        resource::{ResourceMutation, generate_add_update_remove_by_id},
        value::Id,
    };
    let port = |interface: &str, bridge: &str| InterfaceBridgePortCfg {
        interface: interface.into(),
        bridge: bridge.into(),
        ..Default::default()
    };
    let current = [
        InterfaceBridgePortById { id: Id(0), data: port("vxlan-netze", "bridge-caps") },
        InterfaceBridgePortById { id: Id(1), data: port("ether2", "bridge-lan") },
    ];
    let target = [port("ether2", "bridge-caps"), port("vxlan-netze", "bridge-lan")];
    let mutations =
        generate_add_update_remove_by_id(&current, target.iter().map(std::borrow::Cow::Borrowed))
            .collect::<Vec<_>>();
    let changed = mutations
        .iter()
        .flat_map(|m| m.fields.iter().map(|f| String::from_utf8_lossy(&f.key).into_owned()))
        .collect::<Vec<_>>();
    assert_eq!(changed, ["bridge", "bridge"], "{mutations:?}");
    ResourceMutation::sort_mutations_with_provided_dependencies(
        &mutations,
        ["vxlan-netze", "ether2", "bridge-caps", "bridge-lan"]
            .map(|name| (crate::model::ReferenceType::Interface, name.as_bytes().into())),
    )
    .unwrap();
}
