//! `/ip/dhcp-server/network` (`ros_model/ip/DhcpServer.txt`), with the API output of CHR 7.24.5.
use crate::model::{IpDhcpServerNetworkByAddress, IpDhcpServerNetworkByAddressBuilder};
use crate::resource::{CfgResource, DeserializeRosBuilder};

fn network(fields: &[(&[u8], &[u8])]) -> IpDhcpServerNetworkByAddress {
    let mut builder = IpDhcpServerNetworkByAddressBuilder::init(&Default::default());
    for (key, value) in fields {
        builder.append_field(key, Some(value));
    }
    builder.build().unwrap()
}

/// A network with a netmask and `dns-none` set by hand (klr-sw-3) losing them: RouterOS
/// refuses them empty ("an integer required", "must be either yes or no") and `!netmask`
/// (a syntax error); `netmask=0` and `dns-none=no` take them away, and RouterOS then leaves
/// both out of its output.
#[test]
fn dropping_netmask_and_dns_none_writes_their_defaults() {
    let plain = network(&[(b".id", b"*1"), (b"address", b"172.17.1.28/30")]);
    let set = network(&[
        (b".id", b"*1"),
        (b"address", b"172.17.1.28/30"),
        (b"netmask", b"30"),
        (b"dns-none", b"true"),
    ]);
    let fields = plain
        .0
        .changed_values(&set.0)
        .map(|field| {
            format!(
                "{}={}",
                String::from_utf8_lossy(field.key),
                String::from_utf8_lossy(&field.value)
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(fields, ["dns-none=no", "netmask=0"]);
    assert_eq!(plain.0.changed_values(&plain.0).count(), 0);
}

/// klr-sw-1's network losing its `next-server`: one address (a list is refused), empty is
/// refused ("invalid value for argument next-server"), `!next-server` takes it away.
#[test]
fn dropping_next_server_unsets_it() {
    let plain = network(&[(b".id", b"*1"), (b"address", b"10.192.7.0/24")]);
    let set = network(&[(b".id", b"*1"), (b"address", b"10.192.7.0/24"), (b"next-server", b"10.192.7.5")]);
    assert_eq!(set.0.next_server, Some("10.192.7.5".parse().unwrap()));
    let fields = plain
        .0
        .changed_values(&set.0)
        .map(|field| String::from_utf8_lossy(field.key).into_owned())
        .collect::<Vec<_>>();
    assert_eq!(fields, ["!next-server"]);
}
