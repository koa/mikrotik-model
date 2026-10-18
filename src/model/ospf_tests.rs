//! Static decode tests for `/routing/ospf/interface-template` (`ros_model/routing/Ospf.txt`).

use crate::model::{
    RoutingOspfInterfaceTemplateById, RoutingOspfInterfaceTemplateByIdBuilder,
    RoutingOspfInterfaceTemplateType,
};
use crate::resource::{AppendFieldResult, CfgResource, DeserializeRosBuilder};
use std::time::Duration;

/// An inactive template left from an older configuration (berghof-haus-stall, RouterOS 7.23.8,
/// `interfaces=*4` names an interface that is gone): the API reports only these fields, none of
/// cost, the intervals, `instance-id`, `type` or `disabled`. RouterOS's own values stand in for them
/// (`default=`, as a CHR 7.24.5 reports for a template added without them), so the entry
/// decodes as what the diff reads (`ById`, without the read-only `.about`) and can be taken
/// away instead of failing the whole menu.
#[test]
fn decodes_an_inactive_template_reporting_few_fields() {
    let mut builder = RoutingOspfInterfaceTemplateByIdBuilder::init(&Default::default());
    for (key, value) in [
        (&b".id"[..], &b"*0"[..]),
        (b"area", b"backbone-v2"),
        (b"interfaces", b"*4"),
        (b"networks", b"172.16.2.5"),
        (b"priority", b"1"),
        // and the read-only `inactive=true`, `.about=ospf area not active`, which a fetch
        // passes over
    ] {
        let result = builder.append_field(key, Some(value));
        assert!(
            matches!(result, AppendFieldResult::Appended),
            "{} not accepted: {result:?}",
            String::from_utf8_lossy(key)
        );
    }
    let template: RoutingOspfInterfaceTemplateById =
        builder.build().expect("RouterOS's values for the missing fields");
    assert_eq!(template.data.cost, 1);
    assert_eq!(template.data.dead_interval, Duration::from_secs(40));
    assert_eq!(template.data.hello_interval, Duration::from_secs(10));
    assert_eq!(template.data.instance_id, 0);
    assert_eq!(template.data.retransmit_interval, Duration::from_secs(5));
    assert_eq!(template.data.transmit_delay, Duration::from_secs(1));
    assert_eq!(template.data._type, RoutingOspfInterfaceTemplateType::Broadcast);
    assert_eq!(template.data.priority, 1);
    assert!(!template.data.disabled);
}

/// A template that had authentication and BFD (router-berghof-burri, 7.23.8) losing them:
/// RouterOS refuses an empty `auth-id`, `auth` and `use-bfd` (CHR 7.24.5: "an integer
/// required", "ambiguous value", "must be either yes or no") and takes `!auth-id` etc.
#[test]
fn dropping_authentication_unsets_its_fields() {
    let mut builder = RoutingOspfInterfaceTemplateByIdBuilder::init(&Default::default());
    for (key, value) in [
        (&b".id"[..], &b"*1"[..]),
        (b"area", b"backbone-v2"),
        (b"interfaces", b"e01-schopf"),
        (b"auth", b"md5"),
        (b"auth-id", b"1"),
        (b"use-bfd", b"yes"),
        (b"priority", b"128"),
    ] {
        builder.append_field(key, Some(value));
    }
    let current: RoutingOspfInterfaceTemplateById = builder.build().unwrap();
    let mut target = current.data.clone();
    target.auth = None;
    target.auth_id = None;
    target.use_bfd = None;
    let fields = target
        .changed_values(&current.data)
        .map(|field| String::from_utf8_lossy(&field.key).into_owned())
        .collect::<Vec<_>>();
    assert_eq!(fields, ["!auth", "!auth-id", "!use-bfd"]);
}
