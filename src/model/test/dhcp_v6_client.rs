//! `/ipv6/dhcp-client` (`ros_model/ipv6/DhcpClient.txt`), with the API output of CHR 7.24.5.
use crate::model::{
    Ipv6DhcpClient, Ipv6DhcpClientBuilder, Ipv6DhcpClientCfg, Ipv6DhcpClientRequest,
};
use crate::resource::{CfgResource, DeserializeRosBuilder};

fn client(fields: &[(&[u8], &[u8])]) -> Ipv6DhcpClient {
    let mut builder = Ipv6DhcpClientBuilder::init(&Default::default());
    for (key, value) in fields {
        builder.append_field(key, Some(value));
    }
    builder.build().unwrap()
}

/// What every client reports; a client never given `pool-prefix-length` and without a default
/// route reports neither it nor `default-route-distance`.
const PLAIN: [(&[u8], &[u8]); 18] = [
    (b".id", b"*3"),
    (b"interface", b"ether3"),
    (b"status", b"searching..."),
    (b"duid", b"0x00030001525400123456"),
    (b"dhcp-server-v6", b"::"),
    (b"request", b"address"),
    (b"accept-prefix-without-address", b"true"),
    (b"add-default-route", b"false"),
    (b"default-route-tables", b"default"),
    (b"check-gateway", b"none"),
    (b"use-peer-dns", b"true"),
    (b"validate-server-duid", b"true"),
    (b"allow-reconfigure", b"false"),
    (b"dhcp-options", b""),
    (b"pool-name", b""),
    (b"prefix-hint", b"::/0"),
    (b"prefix-address-lists", b""),
    (b"invalid", b"false"),
];

/// A client asking for an address only gets prefix delegation turned on: `request`, the pool
/// and the default route change, the rest stays.
#[test]
fn a_client_without_prefix_delegation_asks_for_a_prefix() {
    let plain = client(&PLAIN);
    assert_eq!(&*plain.cfg.interface, b"ether3".as_slice());
    assert_eq!(plain.cfg.pool_prefix_length, None);
    assert_eq!(plain.cfg.default_route_distance, None);
    assert!(!plain.cfg.disabled);
    let target = Ipv6DhcpClientCfg {
        request: [Ipv6DhcpClientRequest::Address, Ipv6DhcpClientRequest::Prefix].into(),
        add_default_route: true,
        default_route_distance: Some(10),
        pool_name: Some("pd-ether3".into()),
        pool_prefix_length: Some(64),
        ..plain.cfg.clone()
    };
    let fields = target
        .changed_values(&plain.cfg)
        .map(|field| {
            format!(
                "{}={}",
                String::from_utf8_lossy(field.key),
                String::from_utf8_lossy(&field.value)
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        fields,
        [
            "request=address,prefix",
            "add-default-route=yes",
            "default-route-distance=10",
            "pool-name=pd-ether3",
            "pool-prefix-length=64",
        ]
    );
}
