//! `/ip/dhcp-server/lease` (`ros_model/ip/DhcpServer.txt`), with the API output of CHR 7.24.5.
use crate::model::{IpDhcpServerLease, IpDhcpServerLeaseBuilder};
use crate::resource::{AppendFieldResult, CfgResource, DeserializeRosBuilder};

fn lease(fields: &[(&[u8], &[u8])]) -> IpDhcpServerLease {
    let mut builder = IpDhcpServerLeaseBuilder::init(&Default::default());
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

/// What every static lease reports besides its own values.
const COMMON: &[(&[u8], &[u8])] = &[
    (b"address-lists", b""),
    (b"dhcp-option", b""),
    (b"agent-circuit-id", b""),
    (b"agent-remote-id", b""),
    (b"status", b"waiting"),
    (b"last-seen", b"never"),
    (b"active-agent-circuit-id", b""),
    (b"active-agent-remote-id", b""),
    (b"radius", b"false"),
    (b"dynamic", b"false"),
    (b"blocked", b"false"),
    (b"disabled", b"false"),
];

fn with_common(own: &[(&'static [u8], &'static [u8])]) -> Vec<(&'static [u8], &'static [u8])> {
    own.iter().chain(COMMON).copied().collect()
}

fn changes(from: &IpDhcpServerLease, to: &IpDhcpServerLease) -> Vec<String> {
    to.cfg
        .changed_values(&from.cfg)
        .map(|field| {
            format!(
                "{}={}",
                String::from_utf8_lossy(field.key),
                String::from_utf8_lossy(&field.value)
            )
        })
        .collect()
}

/// A lease of one server by MAC, one of all servers (no `server`), one by client ID (no MAC):
/// what RouterOS leaves out is none, false and 0s.
#[test]
fn decodes_the_static_leases_of_routeros() {
    let printer = lease(&with_common(&[
        (b".id", b"*1"),
        (b"address", b"10.10.0.50"),
        (b"mac-address", b"AA:BB:CC:DD:EE:01"),
        (b"server", b"dhcp-test"),
        (b"comment", b"drucker"),
    ]));
    assert_eq!(printer.cfg.server.as_deref(), Some(&b"dhcp-test"[..]));
    assert!(!printer.cfg.block_access && printer.cfg.lease_time.is_zero());
    let all = lease(&with_common(&[
        (b".id", b"*2"),
        (b"address", b"10.10.0.51"),
        (b"mac-address", b"AA:BB:CC:DD:EE:02"),
    ]));
    assert_eq!(all.cfg.server, None);
    let client = lease(&with_common(&[
        (b".id", b"*3"),
        (b"address", b"10.10.0.52"),
        (b"client-id", b"1:aa:bb:cc:dd:ee:03"),
        (b"server", b"dhcp-test"),
    ]));
    assert_eq!(client.cfg.mac_address, None);
}

/// Taking a lease's settings away writes what RouterOS accepts: `no` for the flags (empty is
/// refused), 0s for the lease time (empty is refused), `!queue-type` (empty is ambiguous).
/// A parent queue cleared by "" reports `none`, which is the same as none reported.
#[test]
fn dropping_settings_writes_what_routeros_accepts() {
    let plain = lease(&with_common(&[
        (b".id", b"*4"),
        (b"address", b"10.10.0.53"),
        (b"mac-address", b"AA:BB:CC:DD:EE:04"),
        (b"server", b"dhcp-test"),
    ]));
    let set = lease(&with_common(&[
        (b".id", b"*4"),
        (b"address", b"10.10.0.53"),
        (b"mac-address", b"AA:BB:CC:DD:EE:04"),
        (b"use-src-mac", b"true"),
        (b"server", b"dhcp-test"),
        (b"block-access", b"true"),
        (b"lease-time", b"1h"),
        (b"queue-type", b"default"),
        (b"parent-queue", b"none"),
    ]));
    assert_eq!(
        changes(&set, &plain),
        ["block-access=no", "lease-time=0s", "!queue-type=", "use-src-mac=no"]
    );
    assert!(changes(&plain, &plain).is_empty());
}
