//! Static decode tests for the firewall (`ros_model/ip/Firewall.txt`, `ipv6/Firewall.txt`)
//! and the OSPF instance, with the API output of CHR 7.24.5 and of an SXT updated from
//! RouterOS 6 to 7.23.8.

use crate::model::{
    IpFirewallFilterAction, IpFirewallFilterCfg, IpFirewallFilterCfgBuilder, IpFirewallNatAction,
    IpFirewallNatCfg, IpFirewallNatCfgBuilder, Ipv6FirewallFilterAction, Ipv6FirewallFilterCfg,
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

/// A rule added without `action` doesn't report it either, in the filter and NAT: it accepts
/// (`default=accept`; router-faesigrund-keller's IPv6 filter, CHR 7.24.5).
#[test]
fn rule_added_without_action_accepts() {
    let rule: Ipv6FirewallFilterCfg = build::<Ipv6FirewallFilterCfgBuilder, _>(&[
        (b"chain", b"input"),
        (b"protocol", b"icmpv6"),
    ]);
    assert_eq!(rule.action, Ipv6FirewallFilterAction::Accept);
    let rule: IpFirewallFilterCfg = build::<IpFirewallFilterCfgBuilder, _>(&[
        (b"chain", b"input"),
        (b"protocol", b"icmp"),
    ]);
    assert_eq!(rule.action, IpFirewallFilterAction::Accept);
    let rule: IpFirewallNatCfg = build::<IpFirewallNatCfgBuilder, _>(&[(b"chain", b"srcnat")]);
    assert_eq!(rule.action, IpFirewallNatAction::Accept);
}

/// A stage of the SSH bruteforce protection as CHR 7.24.5 reports it; `10d` comes back as
/// `1w3d`.
#[test]
fn decodes_a_bruteforce_stage() {
    let rule: IpFirewallFilterCfg = build::<IpFirewallFilterCfgBuilder, _>(&[
        (b"chain", b"input"),
        (b"action", b"add-src-to-address-list"),
        (b"connection-state", b"new"),
        (b"protocol", b"tcp"),
        (b"src-address-list", b"ssh_stage3"),
        (b"address-list", b"ssh_blacklist"),
        (b"address-list-timeout", b"1w3d"),
        (b"dst-port", b"22"),
    ]);
    assert_eq!(rule.action, IpFirewallFilterAction::AddSrcToAddressList);
    assert_eq!(rule.address_list_timeout, Some(std::time::Duration::from_secs(10 * 24 * 3600)));
    assert_eq!(rule.src_address_list.as_deref(), Some(b"ssh_stage3".as_slice()));
    let rule: IpFirewallFilterCfg = build::<IpFirewallFilterCfgBuilder, _>(&[
        (b"chain", b"forward"),
        (b"connection-nat-state", b"dstnat"),
    ]);
    assert_eq!(rule.connection_nat_state.as_deref(), Some(b"dstnat".as_slice()));
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

mod owned {
    //! Only the rules whose comment has the prefix are the target's (`generate_owned_by_id`).
    use crate::{
        generator::Generator,
        model::{IpFirewallFilterAction, IpFirewallFilterById, IpFirewallFilterCfg, ReferenceType},
        resource::{ResourceMutation, ResourceMutationOperation, generate_owned_by_id},
        value::Id,
    };
    use std::borrow::Cow;

    fn rule(comment: Option<&str>, port: &str) -> IpFirewallFilterCfg {
        IpFirewallFilterCfg {
            chain: b"input".into(),
            action: IpFirewallFilterAction::Drop,
            comment: comment.map(Into::into),
            in_interface: Some(b"ether1".into()),
            protocol: Some(b"tcp".into()),
            dst_port: Some(port.into()),
            address_list: None,
            address_list_timeout: None,
            connection_nat_state: None,
            connection_state: None,
            disabled: false,
            dst_address: None,
            in_interface_list: None,
            jump_target: None,
            out_interface: None,
            out_interface_list: None,
            src_address: None,
            src_address_list: None,
        }
    }
    fn entry(id: u32, comment: Option<&str>, port: &str) -> IpFirewallFilterById {
        IpFirewallFilterById { id: Id(id), data: rule(comment, port) }
    }
    fn reconcile<'a>(
        current: &'a [IpFirewallFilterById],
        target: &'a [IpFirewallFilterCfg],
    ) -> Vec<ResourceMutation<'a>> {
        generate_owned_by_id(current, target.iter().map(Cow::Borrowed), b"own:", true, |r| {
            r.comment.as_deref()
        })
        .collect()
    }
    fn script(mutations: &[ResourceMutation]) -> String {
        let mut text = String::new();
        let mut generator = Generator::new(&mut text);
        for mutation in mutations {
            generator.append_mutation(mutation).unwrap();
        }
        text
    }

    #[test]
    fn others_stay_and_own_go_first() {
        let current = [
            entry(1, Some("by hand"), "22"),
            entry(2, Some("own: old"), "23"),
            entry(3, Some("defconf: drop"), "80"),
        ];
        let target = [rule(Some("own: dns"), "53")];
        let mutations = reconcile(&current, &target);
        // the own rule is rewritten, the others stay; then it goes before the first other
        assert_eq!(
            script(&mutations),
            "/ip/firewall/filter\n\
             set [ find .id=*2 dynamic!=yes] comment=\"own: dns\" dst-port=53 \n\
             move [find comment=\"own: dns\" dynamic!=yes] destination=*1\n"
        );
    }

    #[test]
    fn rules_in_place_need_nothing() {
        let current = [
            entry(1, Some("own: a"), "22"),
            entry(2, Some("own: b"), "53"),
            entry(3, Some("by hand"), "80"),
        ];
        let target = [rule(Some("own: a"), "22"), rule(Some("own: b"), "53")];
        assert_eq!(script(&reconcile(&current, &target)), "");
    }

    #[test]
    fn own_rules_out_of_order_are_moved() {
        let current = [
            entry(1, Some("own: b"), "53"),
            entry(2, Some("own: a"), "22"),
            entry(3, Some("by hand"), "80"),
        ];
        let target = [rule(Some("own: a"), "22"), rule(Some("own: b"), "53")];
        assert_eq!(
            script(&reconcile(&current, &target)),
            "/ip/firewall/filter\n\
             move [find comment=\"own: a\" dynamic!=yes] destination=*3\n\
             move [find comment=\"own: b\" dynamic!=yes] destination=*3\n"
        );
        // behind another rule
        let current = [entry(3, Some("by hand"), "80"), entry(1, Some("own: a"), "22")];
        let target = [rule(Some("own: a"), "22")];
        assert_eq!(
            script(&reconcile(&current, &target)),
            "/ip/firewall/filter\nmove [find comment=\"own: a\" dynamic!=yes] destination=*3\n"
        );
    }

    /// A rule marked `last` goes after the others, the rest before them (the drop that ends a
    /// chain, after the rules set up by hand).
    #[test]
    fn a_last_rule_goes_after_the_others() {
        let current = [entry(1, Some("by hand"), "80"), entry(2, Some("by hand 2"), "443")];
        let target = [rule(Some("own: ssh"), "22"), rule(Some("own: last drop"), "1-65535")];
        assert_eq!(
            script(&reconcile(&current, &target)),
            "/ip/firewall/filter\n\
             add action=drop chain=input comment=\"own: ssh\" disabled=no dst-port=22 \
             in-interface=ether1 protocol=tcp \n\
             add action=drop chain=input comment=\"own: last drop\" disabled=no \
             dst-port=1-65535 in-interface=ether1 protocol=tcp \n\
             move [find comment=\"own: ssh\" dynamic!=yes] destination=*1\n\
             move [find comment=\"own: last drop\" dynamic!=yes]\n"
        );
        // in place: nothing
        let current = [
            entry(1, Some("own: ssh"), "22"),
            entry(2, Some("by hand"), "80"),
            entry(3, Some("own: last drop"), "1-65535"),
        ];
        assert_eq!(script(&reconcile(&current, &target)), "");
        // without others: still first, then last
        let current = [entry(3, Some("own: last drop"), "1-65535"), entry(1, Some("own: ssh"), "22")];
        assert_eq!(
            script(&reconcile(&current, &target)),
            "/ip/firewall/filter\n\
             move [find comment=\"own: ssh\" dynamic!=yes] destination=*3\n\
             move [find comment=\"own: last drop\" dynamic!=yes]\n"
        );
        // another rule after the last one: it goes to the end again
        let current = [
            entry(1, Some("own: ssh"), "22"),
            entry(3, Some("own: last drop"), "1-65535"),
            entry(2, Some("by hand"), "80"),
        ];
        assert_eq!(
            script(&reconcile(&current, &target)),
            "/ip/firewall/filter\n\
             move [find comment=\"own: ssh\" dynamic!=yes] destination=*2\n\
             move [find comment=\"own: last drop\" dynamic!=yes]\n"
        );
    }

    /// The move to the end waits for the other move, not that for it: no cycle.
    #[test]
    fn the_move_to_the_end_comes_after_the_other() {
        let current = [entry(1, Some("by hand"), "80")];
        let target = [rule(Some("own: ssh"), "22"), rule(Some("own: last drop"), "1-65535")];
        let mutations = reconcile(&current, &target);
        let moves = mutations
            .iter()
            .filter(|m| matches!(m.operation, ResourceMutationOperation::Move { .. }))
            .collect::<Vec<_>>();
        assert_eq!(moves.len(), 2);
        assert!(moves[1].waits_for(moves[0]));
        assert!(!moves[0].waits_for(moves[1]));
    }

    #[test]
    fn a_rule_without_comment_is_taken_over() {
        // written before the prefix: the same rule without a comment
        let current = [entry(1, None, "53"), entry(2, None, "80"), entry(3, Some("by hand"), "22")];
        let target = [rule(Some("own: dns"), "53"), rule(Some("own: ssh"), "22")];
        let mutations = reconcile(&current, &target);
        // the copy of dns is taken over; ssh has a comment of another: added, not taken
        assert_eq!(
            script(&mutations),
            "/ip/firewall/filter\n\
             set [ find .id=*1 dynamic!=yes] comment=\"own: dns\" \n\
             add action=drop chain=input comment=\"own: ssh\" disabled=no dst-port=22 \
             in-interface=ether1 protocol=tcp \n\
             move [find comment=\"own: dns\" dynamic!=yes] destination=*2\n\
             move [find comment=\"own: ssh\" dynamic!=yes] destination=*2\n"
        );
    }

    #[test]
    fn the_move_waits_for_its_menu() {
        let current = [entry(3, Some("by hand"), "80")];
        let mut target = [rule(Some("own: a"), "22")];
        target[0].in_interface = Some(b"vlan-new".into());
        let mutations = reconcile(&current, &target);
        assert!(matches!(mutations[0].operation, ResourceMutationOperation::Add));
        assert!(matches!(mutations[1].operation, ResourceMutationOperation::Move { .. }));
        // the move first in the list: it still runs after the add, which waits for its interface
        let reversed = [mutations[1].clone(), mutations[0].clone()];
        let sorted = ResourceMutation::sort_mutations_with_provided_dependencies(
            &reversed,
            [(ReferenceType::Interface, b"vlan-new".as_slice().into())],
        )
        .unwrap();
        assert!(matches!(sorted[1].operation, ResourceMutationOperation::Move { .. }));
    }
}
