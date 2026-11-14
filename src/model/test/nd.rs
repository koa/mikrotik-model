//! `/ipv6/nd` (`ipv6/Nd.txt`): a new entry gets `advertise-dns` in a `set` after its `add`
//! (`after-add`, `ResourceMutationOperation::AddThenSet`).
use crate::{
    generator::Generator,
    model::{Ipv6NdByInterface, Ipv6NdCfg},
    resource::{Creatable, ResourceMutationOperation},
};

fn entry(advertise_dns: Option<bool>) -> Ipv6NdByInterface {
    let mut cfg = Ipv6NdCfg::new(b"vlan-clients".into(), false);
    cfg.dns = Some(["2001:db8::53".parse().unwrap()].into());
    cfg.advertise_dns = advertise_dns;
    Ipv6NdByInterface(cfg)
}

fn script(entry: &Ipv6NdByInterface) -> String {
    let mut source = String::new();
    Generator::new(&mut source)
        .append_mutation(&entry.calculate_create())
        .unwrap();
    source
}

/// The `add` doesn't carry `advertise-dns`; the `set` of the new entry, found by its
/// interface, does.
#[test]
fn advertise_dns_is_set_after_the_add() {
    let entry = entry(Some(true));
    let create = entry.calculate_create();
    let ResourceMutationOperation::AddThenSet { key, fields } = &create.operation else {
        panic!("expected AddThenSet, got {:?}", create.operation);
    };
    assert_eq!(key.key, b"interface");
    assert_eq!(key.value.as_ref(), b"vlan-clients");
    assert_eq!(
        fields
            .iter()
            .map(|field| (field.key, field.value.as_ref()))
            .collect::<Vec<_>>(),
        [(&b"advertise-dns"[..], &b"yes"[..])]
    );
    assert!(
        create
            .fields
            .iter()
            .all(|field| field.key != b"advertise-dns")
    );
    assert_eq!(
        script(&entry),
        "/ipv6/nd\nset [add interface=vlan-clients dns=\"2001:db8::53\" disabled=no ] advertise-dns=yes \n"
    );
}

/// Without `advertise-dns` it is a plain `add`.
#[test]
fn without_advertise_dns_it_is_a_plain_add() {
    let entry = entry(None);
    assert_eq!(
        entry.calculate_create().operation,
        ResourceMutationOperation::Add
    );
}
