//! Static decode tests for the menus the provisioner only reads (`file.txt`, `log.txt`,
//! `system/Scheduler.txt`, `system/Script.txt`, `system/DeviceMode.txt`, `user.txt`,
//! `ipv6/Route.txt`, `ip/ip.txt`'s `/ip/route`, `system.txt`'s `/system/resource`,
//! `system/Package.txt`, `routing/Ospf.txt`'s neighbors, `partitions.txt`), with what a device
//! answered over the API: CHR 7.24.5, the OSPF neighbors lab-np2's, the partitions lab-sxt-b's
//! (both 7.24.6). A field the model doesn't know fails the test.

use crate::model::{
    File, FileBuilder, IpRoute, IpRouteBuilder, Ipv6RouteState, Ipv6RouteStateBuilder, LogState,
    LogStateBuilder, SystemDeviceModeState, SystemDeviceModeStateBuilder, SystemPackageState,
    SystemPackageStateBuilder, SystemResourceState, SystemResourceStateBuilder, SystemScheduler,
    SystemSchedulerBuilder, SystemScript, SystemScriptBuilder, SystemScriptJobState,
    SystemScriptJobStateBuilder, UserActiveState, UserActiveStateBuilder,
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

#[test]
fn decodes_a_file_and_a_directory() {
    let file: File = build::<FileBuilder, _>(&[
        (b".id", b"**1louYup4CVeg3q2mUXU"),
        (b"name", b"probe.txt"),
        (b"type", b".txt file"),
        (b"size", b"5"),
        (b"last-modified", b"2026-11-13 05:02:34"),
        (b"contents", b"hello"),
    ]);
    assert_eq!(file.status.size, Some(5));
    let directory: File = build::<FileBuilder, _>(&[
        (b".id", b"**NQMkAHLmXTJHX"),
        (b"name", b"skins"),
        (b"type", b"directory"),
        (b"last-modified", b"2026-10-02 15:44:03"),
    ]);
    assert_eq!(directory.status.size, None);
}

#[test]
fn decodes_a_scheduler_entry() {
    let entry: SystemScheduler = build::<SystemSchedulerBuilder, _>(&[
        (b".id", b"*0"),
        (b"name", b"probe"),
        (b"start-time", b"startup"),
        (b"interval", b"1h"),
        (b"on-event", b":log info probe"),
        (b"owner", b"admin"),
        (
            b"policy",
            b"ftp,reboot,read,write,policy,test,password,sniff,sensitive,romon",
        ),
        (b"run-count", b"0"),
        (b"next-run", b"2026-11-13 05:32:21"),
        (b"disabled", b"false"),
    ]);
    assert_eq!(entry.status.run_count, Some(0));
    assert_eq!(
        entry.cfg.interval,
        Some(std::time::Duration::from_secs(3600))
    );
}

#[test]
fn decodes_a_script_and_its_jobs() {
    let _: SystemScript = build::<SystemScriptBuilder, _>(&[
        (b".id", b"*1"),
        (b"name", b"probe-script"),
        (b"owner", b"admin"),
        (
            b"policy",
            b"ftp,reboot,read,write,policy,test,password,sniff,sensitive,romon",
        ),
        (b"dont-require-permissions", b"false"),
        (b"run-count", b"0"),
        (b"source", b":delay 60s"),
        (b"invalid", b"false"),
        (b"comment", b"probe"),
    ]);
    let _: SystemScriptJobState = build::<SystemScriptJobStateBuilder, _>(&[
        (b".id", b"*3B"),
        (b"owner", b"admin"),
        (b"policy", b"local,telnet,ssh,ftp,reboot,read,write,policy,test,winbox,password,web,sniff,sensitive,api,romon,rest-api"),
        (b"parent", b"*39"),
        (b"started", b"2026-11-13 05:02:34"),
        (b"trace", b"api:admin@10.0.2.2"),
        (b"type", b"command"),
    ]);
}

#[test]
fn decodes_the_device_mode() {
    let mode: SystemDeviceModeState = build::<SystemDeviceModeStateBuilder, _>(&[
        (b"mode", b"advanced"),
        (b"allowed-versions", b"7.13+,6.49.8+"),
        (b"flagged", b"false"),
        (b"flagging-enabled", b"true"),
        (b"scheduler", b"true"),
        (b"socks", b"true"),
        (b"fetch", b"true"),
        (b"pptp", b"true"),
        (b"l2tp", b"true"),
        (b"bandwidth-test", b"true"),
        (b"traffic-gen", b"false"),
        (b"sniffer", b"true"),
        (b"ipsec", b"true"),
        (b"romon", b"true"),
        (b"proxy", b"true"),
        (b"hotspot", b"true"),
        (b"smb", b"true"),
        (b"email", b"true"),
        (b"zerotier", b"true"),
        (b"container", b"false"),
        (b"install-any-version", b"false"),
        (b"partitions", b"false"),
        (b"routerboard", b"false"),
        (b"attempt-count", b"0"),
    ]);
    assert_eq!(mode.scheduler, Some(true));
    assert_eq!(mode.partitions, Some(false));
}

#[test]
fn decodes_an_api_session() {
    let _: UserActiveState = build::<UserActiveStateBuilder, _>(&[
        (b".id", b"*B"),
        (b"when", b"2026-11-13 04:33:47"),
        (b"name", b"admin"),
        (b"address", b"10.0.2.2"),
        (b"via", b"api"),
        (b"group", b"full"),
        (b"radius", b"false"),
    ]);
}

#[test]
fn decodes_a_log_entry() {
    let _: LogState = build::<LogStateBuilder, _>(&[
        (b".id", b"*3"),
        (b"time", b"2026-11-13 04:32:35"),
        (b"topics", b"system,info,account"),
        (b"message", b"user admin logged in from 10.0.2.2 via api"),
        (
            b"extra-info",
            b"app=api duser=admin outcome=success src=10.0.2.2 ",
        ),
    ]);
}

#[test]
fn decodes_the_routes() {
    let _: IpRoute = build::<IpRouteBuilder, _>(&[
        (b".id", b"*80000001"),
        (b"dst-address", b"0.0.0.0/0"),
        (b"routing-table", b"main"),
        (b"gateway", b"10.0.2.2"),
        (b"immediate-gw", b"10.0.2.2%ether1"),
        (b"distance", b"1"),
        (b"scope", b"30"),
        (b"target-scope", b"10"),
        (b"vrf-interface", b"ether1"),
        (b"dynamic", b"true"),
        (b"inactive", b"false"),
        (b"active", b"true"),
        (b"dhcp", b"true"),
    ]);
    let _: IpRoute = build::<IpRouteBuilder, _>(&[
        (b".id", b"*20183010"),
        (b"dst-address", b"10.0.2.0/24"),
        (b"routing-table", b"main"),
        (b"gateway", b"ether1"),
        (b"immediate-gw", b"ether1"),
        (b"distance", b"0"),
        (b"scope", b"10"),
        (b"target-scope", b"5"),
        (b"local-address", b"10.0.2.15%ether1"),
        (b"dynamic", b"true"),
        (b"inactive", b"false"),
        (b"active", b"true"),
        (b"connect", b"true"),
    ]);
    let _: Ipv6RouteState = build::<Ipv6RouteStateBuilder, _>(&[
        (b".id", b"*20183020"),
        (b"dst-address", b"fe80::/64"),
        (b"routing-table", b"main"),
        (b"gateway", b"ether1"),
        (b"immediate-gw", b"ether1"),
        (b"distance", b"0"),
        (b"scope", b"10"),
        (b"target-scope", b"5"),
        (b"dynamic", b"true"),
        (b"inactive", b"false"),
        (b"active", b"true"),
        (b"connect", b"true"),
    ]);
}

#[test]
fn decodes_the_resource_and_packages() {
    let resource: SystemResourceState = build::<SystemResourceStateBuilder, _>(&[
        (b"uptime", b"42m44s"),
        (b"version", b"7.24.5 (stable)"),
        (b"build-time", b"2026-09-29 14:21:27"),
        (b"minimum-version", b"7.1"),
        (b"free-memory", b"320348160"),
        (b"total-memory", b"536870912"),
        (b"cpu", b"QEMU"),
        (b"cpu-count", b"2"),
        (b"cpu-frequency", b"4291"),
        (b"cpu-load", b"0"),
        (b"free-hdd-space", b"72429568"),
        (b"total-hdd-space", b"93564928"),
        (b"write-sect-since-reboot", b"1336"),
        (b"write-sect-total", b"1336"),
        (b"architecture-name", b"x86_64"),
        (b"board-name", b"CHR QEMU Standard PC (i440FX + PIIX, 1996)"),
        (b"platform", b"MikroTik"),
    ]);
    assert_eq!(
        resource.architecture_name,
        crate::model::SystemArchitecture::X8664
    );
    let package: SystemPackageState = build::<SystemPackageStateBuilder, _>(&[
        (b".id", b"*1"),
        (b"name", b"routeros"),
        (b"version", b"7.24.5"),
        (b"build-time", b"2026-09-29 14:21:27"),
        (b"scheduled", b""),
        (b"size", b"20841356"),
        (b"available", b"false"),
        (b"disabled", b"false"),
    ]);
    assert!(!package.disabled);
}

#[test]
fn decodes_an_ospf_neighbor() {
    use crate::model::{
        OspfNeighborState, RoutingOspfNeighborState, RoutingOspfNeighborStateBuilder,
    };
    // lab-np2, 7.24.6
    let neighbor: RoutingOspfNeighborState = build::<RoutingOspfNeighborStateBuilder, _>(&[
        (b".id", b"*F3FCD706"),
        (b"instance", b"default-v2"),
        (b"area", b"backbone-v2"),
        (b"interface", b"e05-hex"),
        (b"address", b"192.168.101.30"),
        (b"priority", b"128"),
        (b"router-id", b"192.168.100.9"),
        (b"dr", b"192.168.101.30"),
        (b"bdr", b"192.168.101.29"),
        (b"state", b"Full"),
        (b"state-changes", b"5"),
        (b"adjacency", b"6d16h26m18s"),
        (b"timeout", b"38s"),
        (b"dynamic", b"true"),
    ]);
    assert_eq!(neighbor.state, OspfNeighborState::Full);
    let v3: RoutingOspfNeighborState = build::<RoutingOspfNeighborStateBuilder, _>(&[
        (b".id", b"*F3FCD79C"),
        (b"instance", b"default-v3"),
        (b"area", b"backbone-v3"),
        (b"interface", b"e15"),
        (b"address", b"fe80::4e5e:cff:fe17:ec1b"),
        (b"router-id", b"172.16.1.1"),
        (b"state", b"TwoWay"),
    ]);
    assert_eq!(v3.state, OspfNeighborState::TwoWay);
}

#[test]
fn decodes_a_partition() {
    use crate::model::{PartitionsState, PartitionsStateBuilder};
    // lab-sxt-b, 7.24.6
    let partition: PartitionsState = build::<PartitionsStateBuilder, _>(&[
        (b".id", b"*2"),
        (b"name", b"part1"),
        (b"fallback-to", b"next"),
        (b"version", b"RouterOS v7.24.6 2026-10-30 08:24:29"),
        (b"size", b"64"),
        (b"active", b"true"),
        (b"running", b"true"),
    ]);
    assert_eq!(partition.size, Some(64));
}

#[test]
fn decodes_a_certificate_and_a_tls_service() {
    use crate::model::{Certificate, CertificateBuilder, IpService, IpServiceBuilder};
    let certificate: Certificate = build::<CertificateBuilder, _>(&[
        (b".id", b"*1"),
        (b"name", b"probe-ca"),
        (b"trust-store", b"all"),
        (b"common-name", b"probe-ca"),
        (b"subject-alt-name", b""),
        (b"akid", b""),
        (b"skid", b"41A9A6C951D08A6FD1C038A2ECB5DAED511D66E4"),
        (b"expires-after", b"52w23h59m59s"),
        (b"private-key", b"true"),
        (b"authority", b"true"),
        (b"issued", b"false"),
        (b"trusted", b"true"),
    ]);
    assert_eq!(certificate.status.private_key, Some(true));
    let service: IpService = build::<IpServiceBuilder, _>(&[
        (b".id", b"*6"),
        (b"name", b"www-ssl"),
        (b"available-from", b""),
        (b"certificate", b"none"),
        (b"dynamic", b"false"),
        (b"disabled", b"true"),
    ]);
    assert_eq!(
        service.cfg.certificate,
        Some(crate::value::HasNone::NoneValue)
    );
}
