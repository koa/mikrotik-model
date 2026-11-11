//! Scope: Static decode tests of the CRS1xx/2xx switch menus
//! (`/interface/ethernet/switch` and its `vlan`, `egress-vlan-tag`, `ingress-vlan-translation`);
//! nothing
//! else.
//!
//! The rows as lab-crs125 (CRS125-24G-1S, RouterOS 7.14.1) reports them over the API: the
//! dynamic entries of its default bridge, without the fields a CRS1xx doesn't report.

use crate::model::{
    InterfaceEthernetSwitch, InterfaceEthernetSwitchBuilder, InterfaceEthernetSwitchEgressVlanTag,
    InterfaceEthernetSwitchEgressVlanTagBuilder, InterfaceEthernetSwitchIngressVlanTranslation,
    InterfaceEthernetSwitchIngressVlanTranslationBuilder, InterfaceEthernetSwitchVlan,
    InterfaceEthernetSwitchVlanBuilder,
};
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

const PORTS: &[u8] = b"ether1,ether2,ether3,sfp1";

#[test]
fn decodes_a_crs1xx_switch_vlan() {
    let vlan: InterfaceEthernetSwitchVlan = build::<InterfaceEthernetSwitchVlanBuilder, _>(&[
        (b"disabled", b"false"),
        (b"ports", b"ether1,ether2,ether3,sfp1,switch1-cpu"),
        (b"dynamic", b"true"),
        (b"flood", b"false"),
        (b"svl", b"false"),
        (b"vlan-id", b"4095"),
        (b"learn", b"false"),
        (b"ingress-mirror", b"false"),
        (b"qos-group", b"none"),
    ]);
    assert_eq!(vlan.cfg.vlan_id, 4095);
    assert_eq!(vlan.cfg.ports.len(), 5);
    assert_eq!(vlan.cfg.switch, None);
}

#[test]
fn decodes_a_crs1xx_egress_vlan_tag() {
    let tag: InterfaceEthernetSwitchEgressVlanTag =
        build::<InterfaceEthernetSwitchEgressVlanTagBuilder, _>(&[
            (b".id", b"*1"),
            (b"dynamic", b"true"),
            (b"disabled", b"false"),
            (b"tagged-ports", b""),
            (b"vlan-id", b"4095"),
        ]);
    assert_eq!(tag.cfg.vlan_id, 4095);
    assert!(tag.cfg.tagged_ports.is_empty());
}

#[test]
fn decodes_a_crs1xx_ingress_vlan_translation() {
    let translation: InterfaceEthernetSwitchIngressVlanTranslation =
        build::<InterfaceEthernetSwitchIngressVlanTranslationBuilder, _>(&[
            (b".id", b"*1"),
            (b"sa-learning", b"false"),
            (b"customer-vlan-format", b"any"),
            (b"pcp-propagation", b"false"),
            (b"service-vlan-format", b"any"),
            (b"ports", PORTS),
            (b"dynamic", b"true"),
            (b"disabled", b"false"),
            (b"new-customer-vid", b"4095"),
        ]);
    assert_eq!(translation.cfg.new_customer_vid, Some(4095));
    assert_eq!(translation.cfg.customer_vid, None);
    assert_eq!(translation.cfg.ports.len(), 4);
}

/// The switch itself as a CRS1xx reports it (one, `is-single`): none of the fields of the
/// other chips' switches (`switch-all-ports`, `mirror-source`), the ports whose frames outside
/// their VLANs it drops.
#[test]
fn decodes_a_crs1xx_switch() {
    let switch: InterfaceEthernetSwitch = build::<InterfaceEthernetSwitchBuilder, _>(&[
        (b"name", b"switch1"),
        (b"type", b"QCA-8513L"),
        (
            b"drop-if-invalid-or-src-port-not-member-of-vlan-on-ports",
            b"",
        ),
    ]);
    assert_eq!(
        switch
            .cfg
            .drop_if_invalid_or_src_port_not_member_of_vlan_on_ports,
        Some(Default::default())
    );
    assert_eq!(switch.cfg.switch_all_ports, None);
}
