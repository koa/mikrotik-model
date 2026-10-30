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
        .flat_map(|m| m.fields.iter().map(|f| String::from_utf8_lossy(f.key).into_owned()))
        .collect::<Vec<_>>();
    assert_eq!(changed, ["bridge", "bridge"], "{mutations:?}");
    ResourceMutation::sort_mutations_with_provided_dependencies(
        &mutations,
        ["vxlan-netze", "ether2", "bridge-caps", "bridge-lan"]
            .map(|name| (crate::model::ReferenceType::Interface, name.as_bytes().into())),
    )
    .unwrap();
}

/// With IGMP snooping a port reports `multicast-router` twice, its setting first and its state
/// (`false`) further on (lab-core, 7.24.5): the setting counts, or the diff sets it forever.
#[test]
fn a_ports_multicast_router_is_its_setting() {
    use crate::{
        model::{InterfaceBridgePortById, InterfaceBridgePortMulticastRouter},
        resource::SentenceResult,
    };
    use mikrotik_api::prelude::ParsedMessage;
    // the port as lab-core reports it, without `debug-info`
    let reported = ".id=*0 interface=vxlan-lab-wlan bridge=bridge priority=0x80 edge=auto \
        point-to-point=auto learn=auto horizon=none hw=true auto-isolate=false restricted-role=false \
        restricted-tcn=false pvid=1 frame-types=admit-only-vlan-tagged ingress-filtering=true \
        unknown-unicast-flood=true unknown-multicast-flood=true broadcast-flood=true \
        tag-stacking=false bpdu-guard=false trusted=false trusted-ra=false trusted-dhcpv6=false \
        mvrp-registrar-state=normal mvrp-applicant-state=normal-participant \
        multicast-router=temporary-query fast-leave=false status=in-bridge role=designated-port \
        edge-port=false point-to-point-port=true learning=true forwarding=true \
        actual-path-cost=20000 multicast-router=false hw-offload=false inactive=false \
        dynamic=false disabled=false";
    let sentence = reported
        .split_whitespace()
        .map(|field| {
            let (key, value) = field.split_once('=').unwrap();
            (key.as_bytes(), Some(value.as_bytes()))
        })
        .collect::<Vec<_>>();
    let parsed =
        SentenceResult::<InterfaceBridgePortById>::parse_message(&sentence, &Default::default());
    let SentenceResult::Row { value, .. } = &parsed else {
        panic!("not a row: {parsed:?}");
    };
    assert_eq!(value.data.multicast_router, InterfaceBridgePortMulticastRouter::TemporaryQuery);
}

/// ap-klr-solar (7.23.7) snooped, and the target with snooping off set the multicast settings
/// to `""`, which RouterOS refuses ("ambiguous value of igmp-version"; CHR 7.24.5 too). Without
/// snooping it hides them and keeps them, so they are required with RouterOS's values: turning
/// snooping off sets only `igmp-snooping`, and a bridge reporting none of them has them.
#[test]
fn snooping_goes_off_without_clearing_the_multicast_settings() {
    use crate::{
        model::InterfaceBridgeCfg,
        resource::generate_add_update_remove_by_key,
    };
    use std::borrow::Cow;
    let mut snooping = BRIDGE_WITHOUT_MVRP.to_vec();
    snooping.retain(|(key, _)| *key != b"igmp-snooping".as_slice());
    snooping.extend([
        (b"igmp-snooping".as_slice(), b"true".as_slice()),
        (b"multicast-router", b"temporary-query"),
        (b"multicast-querier", b"false"),
        (b"startup-query-count", b"2"),
        (b"last-member-query-count", b"2"),
        (b"last-member-interval", b"1s"),
        (b"membership-interval", b"4m20s"),
        (b"querier-interval", b"4m15s"),
        (b"query-interval", b"2m5s"),
        (b"query-response-interval", b"10s"),
        (b"startup-query-interval", b"31s250ms"),
        (b"igmp-version", b"2"),
        (b"mld-version", b"1"),
    ]);
    let current: InterfaceBridgeByName = build::<InterfaceBridgeByNameBuilder, _>(&snooping);
    let hidden: InterfaceBridgeByName = build::<InterfaceBridgeByNameBuilder, _>(BRIDGE_WITHOUT_MVRP);
    let mut snooped = hidden.clone();
    snooped.0.snoop_multicast();
    assert_eq!(current, snooped);

    let target = InterfaceBridgeByName(InterfaceBridgeCfg {
        igmp_snooping: false,
        ..current.0.clone()
    });
    assert_eq!(target, hidden);
    let current = [current];
    let mutations = generate_add_update_remove_by_key(&current, [Cow::Borrowed(&target)])
        .collect::<Vec<_>>();
    let changed = mutations
        .iter()
        .flat_map(|m| m.fields.iter().map(|f| String::from_utf8_lossy(f.key).into_owned()))
        .collect::<Vec<_>>();
    assert_eq!(changed, ["igmp-snooping"], "{mutations:?}");
}
