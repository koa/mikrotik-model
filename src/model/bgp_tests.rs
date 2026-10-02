//! Static decode tests for the `/routing/bgp/*` and `/routing/filter/rule` entities
//! (`ros_model/routing/Bgp.txt`, `ros_model/routing/Filter.txt`).
//!
//! These feed `append_field` directly with the exact attribute names/values seen in
//! real production RouterOS configs (`netbox-provisioner/examples/`), standing in for
//! `inspect-device` verification against a live device, which isn't reachable from
//! this environment. Note: a `.rsc` *export* omits fields at their default value for
//! readability (e.g. a `vrf=main` instance may print without `vrf=` at all), but the
//! live binary API that `{Name}Current::fetch` actually uses always returns every
//! field resolved - so these tests supply the resolved value even where a source
//! example's exported text omitted it, and use fully-qualified attribute names (the
//! `.network=`/`.as=` leading-dot continuation syntax is an `.rsc` text-export
//! shorthand, never sent over the wire).

use crate::ascii::AsciiString;
use crate::model::{
    RoutingBgpConnection, RoutingBgpConnectionAfi, RoutingBgpConnectionBuilder,
    RoutingBgpConnectionLocalRole, RoutingBgpInstance, RoutingBgpInstanceBuilder,
    RoutingFilterRule, RoutingFilterRuleBuilder, RoutingRedistribute,
};
use crate::resource::{DeserializeRosBuilder, DeserializeRosResource};
use crate::value::{IpOrInterface, RosValue};
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv6Addr};

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

#[test]
fn decodes_real_bgp_instance_with_vrf() {
    // examples/01-core-router-ospf/config.rsc:
    // `/routing bgp instance add as=4200000008 name=bgp-instance-1 vrf=main`
    let instance: RoutingBgpInstance = build::<RoutingBgpInstanceBuilder, _>(&[
        (b"as", b"4200000008"),
        (b"disabled", b"no"),
        (b"name", b"bgp-instance-1"),
        (b"vrf", b"main"),
        (b"about", b""),
        (b"inactive", b"false"),
    ]);
    assert_eq!(instance.cfg._as, 4_200_000_008);
    assert_eq!(instance.cfg.name, AsciiString::from("bgp-instance-1"));
    assert_eq!(instance.cfg.vrf, AsciiString::from("main"));
}

#[test]
fn decodes_real_bgp_instance_with_two_byte_asn_and_no_vrf_in_export() {
    // examples/09-wlan-controller/config.rsc:
    // `/routing bgp instance add as=65580 disabled=no name=bgp-backup-link`
    // (no `vrf=` in the .rsc export - it's at the default "main", which the live API
    // would still report explicitly; supplied here to reflect that, see module docs)
    let instance: RoutingBgpInstance = build::<RoutingBgpInstanceBuilder, _>(&[
        (b"as", b"65580"),
        (b"disabled", b"no"),
        (b"name", b"bgp-backup-link"),
        (b"vrf", b"main"),
        (b"about", b""),
        (b"inactive", b"false"),
    ]);
    assert_eq!(instance.cfg._as, 65_580);
    assert!(!instance.cfg.disabled);
    assert_eq!(instance.cfg.name, AsciiString::from("bgp-backup-link"));
}

#[test]
fn decodes_real_bgp_connection_with_ipv6_link_local_scoped_address() {
    // examples/01-core-router-ospf/config.rsc, the one *active* (non-disabled) peer:
    // `add afi=ipv6 disabled=no instance=bgp-instance-1 local.role=ebgp multihop=yes
    //  name=bgp-hegglin-v6 output.default-originate=if-installed
    //  output.network=bgp-networks remote.address=fe80::...%e02-hegglin
    //  routing-table=main templates=default use-bfd=yes`
    // The scoped IPv6 remote address is the reason `remote.address`/`local.address`
    // use `IpOrInterface`, not a bare `IpAddr` (std's IpAddr/Ipv6Addr FromStr rejects
    // the `%zone` suffix, which real link-local eBGP peers over a radio/ethernet link
    // use throughout this network, e.g. examples/01, 04, 07).
    let connection: RoutingBgpConnection = build::<RoutingBgpConnectionBuilder, _>(&[
        (b"afi", b"ipv6"),
        (b"connect", b"yes"),
        (b"disabled", b"no"),
        (b"instance", b"bgp-instance-1"),
        (b"listen", b"yes"),
        (b"local.role", b"ebgp"),
        (b"multihop", b"yes"),
        (b"name", b"bgp-hegglin-v6"),
        (b"output.default-originate", b"if-installed"),
        (b"output.network", b"bgp-networks"),
        (
            b"remote.address",
            b"fe80::ce2d:e0ff:fe48:99a3%e02-hegglin",
        ),
        (b"routing-table", b"main"),
        (b"templates", b"default"),
        (b"use-bfd", b"yes"),
        (b"about", b""),
        (b"dynamic", b"false"),
        (b"inactive", b"false"),
        (b"local.default-address", b""),
    ]);
    assert_eq!(connection.cfg.afi, RoutingBgpConnectionAfi::Ipv6);
    assert!(!connection.cfg.disabled);
    assert_eq!(connection.cfg.instance, AsciiString::from("bgp-instance-1"));
    assert_eq!(
        connection.cfg.local_role,
        RoutingBgpConnectionLocalRole::Ebgp
    );
    assert!(connection.cfg.multihop);
    assert_eq!(
        connection.cfg.remote_address,
        IpOrInterface::IpWithInterface(crate::value::IpWithInterface {
            ip: IpAddr::V6("fe80::ce2d:e0ff:fe48:99a3".parse::<Ipv6Addr>().unwrap()),
            interface: b"e02-hegglin".as_slice().into(),
        })
    );
    assert!(connection.cfg.use_bfd);
    assert_eq!(
        connection.cfg.templates,
        BTreeSet::from([AsciiString::from("default")])
    );
}

#[test]
fn decodes_real_bgp_connection_with_connection_level_as_override() {
    // examples/09-wlan-controller/config.rsc:
    // `add as=65580 disabled=yes instance=bgp-backup-link local.role=ebgp name=bgp1
    //  output.redistribute=ospf remote.address=10.195.16.1 remote.as=4200000000
    //  routing-table=main`
    // (`.as=4200000000` in the raw export is the dotted-continuation form of
    // `remote.as=4200000000`, inheriting the `remote.` prefix from the immediately
    // preceding `remote.address=` attribute - not related to the separate, genuine
    // top-level `as` field, which overrides the instance's AS for this one session.)
    let connection: RoutingBgpConnection = build::<RoutingBgpConnectionBuilder, _>(&[
        (b"afi", b"ip"),
        (b"as", b"65580"),
        (b"connect", b"yes"),
        (b"disabled", b"yes"),
        (b"instance", b"bgp-backup-link"),
        (b"listen", b"yes"),
        (b"local.role", b"ebgp"),
        (b"multihop", b"no"),
        (b"name", b"bgp1"),
        (b"output.redistribute", b"ospf"),
        (b"remote.address", b"10.195.16.1"),
        (b"remote.as", b"4200000000"),
        (b"routing-table", b"main"),
        (b"use-bfd", b"no"),
        (b"about", b""),
        (b"dynamic", b"false"),
        (b"inactive", b"true"),
        (b"local.default-address", b""),
    ]);
    assert_eq!(connection.cfg._as, Some(65_580));
    assert!(connection.cfg.disabled);
    assert_eq!(connection.cfg.remote_as, Some(4_200_000_000));
    assert_eq!(
        connection.cfg.remote_address,
        IpOrInterface::Ip("10.195.16.1".parse::<IpAddr>().unwrap())
    );
    assert_eq!(
        connection.cfg.output_redistribute,
        BTreeSet::from([RoutingRedistribute::Ospf])
    );
}

#[test]
fn decodes_real_bgp_connection_with_minimal_fields() {
    // examples/07-wlan-ap-capsman-legacy/config.rsc:
    // `add afi=ip disabled=yes instance=bgp-instance-1 local.role=ebgp
    //  name=bgp-klr-sw2 remote.address=172.17.1.5 templates=default`
    let connection: RoutingBgpConnection = build::<RoutingBgpConnectionBuilder, _>(&[
        (b"afi", b"ip"),
        (b"connect", b"yes"),
        (b"disabled", b"yes"),
        (b"instance", b"bgp-instance-1"),
        (b"listen", b"yes"),
        (b"local.role", b"ebgp"),
        (b"multihop", b"no"),
        (b"name", b"bgp-klr-sw2"),
        (b"remote.address", b"172.17.1.5"),
        (b"routing-table", b"main"),
        (b"templates", b"default"),
        (b"use-bfd", b"no"),
        (b"about", b""),
        (b"dynamic", b"false"),
        (b"inactive", b"true"),
        (b"local.default-address", b""),
    ]);
    assert_eq!(connection.cfg.afi, RoutingBgpConnectionAfi::Ip);
    assert_eq!(connection.cfg.name, AsciiString::from("bgp-klr-sw2"));
    assert_eq!(
        connection.cfg.remote_address,
        IpOrInterface::Ip("172.17.1.5".parse::<IpAddr>().unwrap())
    );
}

#[test]
fn decodes_all_live_verified_local_role_values() {
    // Checked against a live RouterOS 7.24.5 CHR instance's `/console/inspect` schema
    // dump (not seen in any example config): `local.role` has 8 variants, not just
    // the `ebgp`/`ibgp` this project actually generates. A device holding a
    // connection with one of the other 6 (e.g. an internet-exchange route-server
    // peering) must still decode correctly - an incomplete enum here would make
    // `{Name}Current::fetch` fail entirely for such a device, not just lose data.
    for role in [
        "ebgp",
        "ebgp-customer",
        "ebgp-peer",
        "ebgp-provider",
        "ebgp-rs",
        "ebgp-rs-client",
        "ibgp",
        "ibgp-rr",
    ] {
        assert!(
            matches!(
                RoutingBgpConnectionLocalRole::parse_ros(role.as_bytes()),
                crate::value::ParseRosValueResult::Value(_)
            ),
            "local.role={role} should be a recognized value"
        );
    }
}

#[test]
fn decodes_real_filter_rule() {
    // examples/09-wlan-controller/config.rsc:
    // `/routing filter rule add chain=ospf-out disabled=no
    //  rule="if (dst in 172.31.80.0/20 && dst-len in 20-32) { accept; }"`
    let rule: RoutingFilterRule = build::<RoutingFilterRuleBuilder, _>(&[
        (b".id", b"*1"),
        (b"chain", b"ospf-out"),
        (b"disabled", b"no"),
        (
            b"rule",
            b"if (dst in 172.31.80.0/20 && dst-len in 20-32) { accept; }",
        ),
        (b"about", b""),
        (b"inactive", b"false"),
    ]);
    assert_eq!(rule.cfg.chain, AsciiString::from("ospf-out"));
    assert!(!rule.cfg.disabled);
    assert_eq!(
        rule.cfg.rule,
        Some(AsciiString::from(
            "if (dst in 172.31.80.0/20 && dst-len in 20-32) { accept; }"
        ))
    );
}
