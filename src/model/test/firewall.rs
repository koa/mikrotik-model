//! Static decode tests for the firewall (`ros_model/ip/Firewall.txt`, `ipv6/Firewall.txt`)
//! and the OSPF instance, with the API output of CHR 7.24.5 and of an SXT updated from
//! RouterOS 6 to 7.23.8.

use crate::model::{
    IpFirewallFilterCfg, IpFirewallFilterCfgBuilder, Ipv6FirewallFilterCfg,
    Ipv6FirewallFilterCfgBuilder, RoutingOspfInstanceCfg, RoutingOspfInstanceCfgBuilder,
};
use crate::resource::{AppendFieldResult, DeserializeRosBuilder, DeserializeRosResource};

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
            matches!(result, AppendFieldResult::Appended),
            "field {} = {} was not accepted: {:?}",
            String::from_utf8_lossy(key),
            String::from_utf8_lossy(value),
            result
        );
    }
    builder.build().expect("all required fields were supplied")
}

/// A rule added without `disabled` doesn't report it (`default=false`).
#[test]
fn rule_added_without_disabled_is_enabled() {
    let rule: IpFirewallFilterCfg = build::<IpFirewallFilterCfgBuilder, _>(&[
        (b"chain", b"forward"),
        (b"action", b"fasttrack-connection"),
        (b"connection-state", b"established,related"),
        (b"comment", b"defconf: fasttrack"),
    ]);
    assert!(!rule.disabled);
}

/// The match fields of the IPv6 defconf rules are ranges, lists, negations and
/// `value:mode` pairs, kept as RouterOS text.
#[test]
fn decodes_the_ipv6_defconf_rules() {
    let rules: [&[(&[u8], &[u8])]; 5] = [
        &[(b"port", b"33434-33534"), (b"protocol", b"udp")],
        &[(b"dst-port", b"500,4500"), (b"src-address", b"!fe80::/10")],
        &[(b"ipsec-policy", b"in,ipsec")],
        &[(b"hop-limit", b"equal:1"), (b"protocol", b"icmpv6")],
        &[(b"protocol", b"139"), (b"limit", b"10,5:packet")],
    ];
    for fields in rules {
        let mut all: Vec<(&[u8], &[u8])> = vec![(b"chain", b"input"), (b"action", b"accept")];
        all.extend_from_slice(fields);
        let rule: Ipv6FirewallFilterCfg = build::<Ipv6FirewallFilterCfgBuilder, _>(&all);
        assert_eq!(&*rule.chain, b"input".as_slice());
    }
}

/// The instance RouterOS 6's OSPF became on the update to 7 reports no `vrf`.
#[test]
fn instance_from_routeros_6_is_in_main() {
    let instance: RoutingOspfInstanceCfg = build::<RoutingOspfInstanceCfgBuilder, _>(&[
        (b"name", b"default-v2"),
        (b"version", b"2"),
        (b"router-id", b"192.168.100.7"),
        (b"disabled", b"false"),
    ]);
    assert_eq!(&*instance.vrf, b"main".as_slice());
}

/// A defconf rule paired with one of the target that has no interface list loses it by
/// `!in-interface-list`: RouterOS refuses `in-interface-list=""` ("ambiguous value", with more
/// than one list; with one it would match that one). Every match field the API refuses empty
/// takes `!field`, checked on rules of a CHR 7.24.5 (set, then `!field`, then read empty).
#[test]
fn a_rule_losing_its_match_fields_unsets_them() {
    use crate::resource::CfgResource;
    let base: &[(&[u8], &[u8])] = &[(b"chain", b"input"), (b"action", b"drop")];
    let plain: IpFirewallFilterCfg = build::<IpFirewallFilterCfgBuilder, _>(base);
    let defconf: IpFirewallFilterCfg = build::<IpFirewallFilterCfgBuilder, _>(
        &[
            base,
            &[
                (b"in-interface-list", b"!LAN"),
                (b"protocol", b"tcp"),
                (b"src-address", b"10.0.0.0/8"),
            ],
        ]
        .concat(),
    );
    let mut keys = plain
        .changed_values(&defconf)
        .map(|field| {
            assert!(field.value.is_empty(), "{}", String::from_utf8_lossy(field.key));
            String::from_utf8_lossy(field.key).into_owned()
        })
        .collect::<Vec<_>>();
    keys.sort();
    assert_eq!(keys, ["!in-interface-list", "!protocol", "!src-address"]);
}
