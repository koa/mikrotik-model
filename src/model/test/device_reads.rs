//! Static decode tests for the menus the provisioner only reads (`file.txt`, `log.txt`,
//! `system/Scheduler.txt`, `system/Script.txt`, `system/DeviceMode.txt`, `user.txt`,
//! `ipv6/Route.txt`, `ip/ip.txt`'s `/ip/route`, `system.txt`'s `/system/resource`,
//! `system/Package.txt`, `routing/Ospf.txt`'s neighbors, interfaces and LSAs,
//! `system/Health.txt`, `partitions.txt`, `ip/Firewall.txt`'s IPv4 address list, `ip/ip.txt`'s DHCP client), with what a device answered over the API: CHR
//! 7.24.5, the OSPF neighbors, interfaces, LSAs and health lab-np2's, the partitions
//! lab-sxt-b's (both 7.24.6). A field the model doesn't know fails the test.

use crate::model::{
    DeviceMode, File, FileBuilder, IpDhcpClient, IpDhcpClientBuilder, IpFirewallAddressList,
    IpFirewallAddressListBuilder, IpRoute, IpRouteBuilder, Ipv6Route, Ipv6RouteBuilder,
    OspfInterfaceState, OspfInterfaceType, OspfLsaType, RoutingOspfInterfaceState,
    RoutingOspfInterfaceStateBuilder, RoutingOspfLsaState, RoutingOspfLsaStateBuilder,
    SystemDeviceModeState, SystemDeviceModeStateBuilder, SystemHealthState,
    SystemHealthStateBuilder, SystemPackageState, SystemPackageStateBuilder, SystemResourceState,
    SystemResourceStateBuilder, SystemScheduler, SystemSchedulerBuilder, SystemScript,
    SystemScriptBuilder, SystemScriptJobState, SystemScriptJobStateBuilder, UserActiveState,
    UserActiveStateBuilder,
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
    assert_eq!(mode.mode, DeviceMode::Advanced);
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
    let _: Ipv6Route = build::<Ipv6RouteBuilder, _>(&[
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

#[test]
fn decodes_the_service_entry_of_an_open_connection() {
    use crate::model::{IpService, IpServiceBuilder};
    let service: IpService = build::<IpServiceBuilder, _>(&[
        (b".id", b"*C"),
        (b"name", b"api"),
        (b"local", b"10.0.2.15"),
        (b"remote", b"10.0.2.2:34720"),
        (b"dynamic", b"true"),
        (b"connection", b"true"),
        (b"disabled", b"false"),
    ]);
    assert!(service.status.dynamic);
    assert_eq!(
        service
            .status
            .remote
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("10.0.2.2:34720")
    );
}

/// `calculate_create` leaves out the fields without a value: RouterOS keeps its defaults there
/// (a script added with `policy=""` would have no rights, a flag refuses the empty value).
#[test]
fn an_entry_built_in_code_is_added_with_its_given_fields_only() {
    use crate::{
        model::SystemScriptCfg,
        resource::{Creatable, ResourceMutationOperation},
    };
    let mut script = SystemScriptCfg::new("apply".into());
    script.source = Some(":put done".into());
    let add = script.calculate_create();
    assert!(matches!(add.operation, ResourceMutationOperation::Add));
    let keys = add.fields.iter().map(|field| field.key).collect::<Vec<_>>();
    assert_eq!(keys, [&b"name"[..], b"source"]);
}

#[test]
fn decodes_a_renamed_port() {
    use crate::model::InterfaceStateBuilder;
    let port = build::<InterfaceStateBuilder, _>(&[
        (b".id", b"*2"),
        (b"name", b"e01"),
        (b"default-name", b"ether1"),
        (b"type", b"ether"),
        (b"running", b"true"),
    ]);
    assert_eq!(
        port.default_name
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("ether1")
    );
}

#[test]
fn decodes_a_log_entry() {
    use crate::{
        model::{LogStateBuilder, LogTopic},
        value::{LogTime, TimeOfDay},
    };
    let entry = build::<LogStateBuilder, _>(&[
        (b".id", b"*26"),
        (b"time", b"2026-11-13 05:02:34"),
        (b"topics", b"system,info,account,container"),
        (b"message", b"user admin logged in from 10.0.2.2 via api"),
        (
            b"extra-info",
            b"app=api duser=admin outcome=success src=10.0.2.2 ",
        ),
    ]);
    assert_eq!(
        entry.time,
        LogTime::At {
            year: 2026,
            month: 11,
            day: 13,
            time: TimeOfDay {
                hour: 5,
                minute: 2,
                second: 34
            }
        }
    );
    assert!(entry.topics.contains(&LogTopic::Account));
    assert!(entry.topics.contains(&LogTopic::Value("container".into())));
    assert_eq!(entry.time.to_string(), "2026-11-13 05:02:34");
}

#[test]
fn reads_the_short_log_times_of_routeros_6() {
    use crate::value::{LogTime, ParseRosValueResult, RosValue};
    for text in ["nov/13 04:32:21", "nov/13/2025 04:32:21", "04:32:21"] {
        let ParseRosValueResult::Value(time) = LogTime::parse_ros(text.as_bytes()) else {
            panic!("{text} not read");
        };
        let written = time.to_string();
        assert_eq!(
            written,
            match text {
                "nov/13/2025 04:32:21" => "2025-11-13 04:32:21",
                text => text,
            }
        );
    }
    assert!(matches!(
        LogTime::parse_ros(b"yesterday"),
        ParseRosValueResult::Invalid
    ));
}

#[test]
fn decodes_the_poe_monitor_of_a_port_off_and_one_feeding() {
    use crate::model::{InterfaceEthernetPoeMonitorBuilder, InterfaceEthernetPoePoeOutStatus};
    // lab-np2 (CRS318, 7.23.8); `power-cycle-host-alive` isn't modeled
    let off = build::<InterfaceEthernetPoeMonitorBuilder, _>(&[
        (b"name", b"e01"),
        (b"poe-out", b"off"),
        (b"poe-voltage", b"auto"),
        (b"poe-out-status", b"disabled"),
    ]);
    assert_eq!(
        off.poe_out_status,
        InterfaceEthernetPoePoeOutStatus::Disabled
    );
    let feeding = build::<InterfaceEthernetPoeMonitorBuilder, _>(&[
        (b"name", b"e03-lab-wap"),
        (b"poe-out", b"auto-on"),
        (b"poe-voltage", b"auto"),
        (b"poe-out-status", b"powered-on"),
        (b"poe-out-voltage", b"47.7"),
        (b"poe-out-current", b"52"),
        (b"poe-out-power", b"2.4"),
        (b"poe-out-power-pair", b"b"),
    ]);
    assert_eq!(
        feeding.poe_out_status,
        InterfaceEthernetPoePoeOutStatus::PoweredOn
    );
    assert_eq!(feeding.poe_out_power, Some(2.4));
}

#[test]
fn decodes_the_lsas() {
    let external: RoutingOspfLsaState = build::<RoutingOspfLsaStateBuilder, _>(&[
        (b"instance", b"default-v2"),
        (b"type", b"external"),
        (b"originator", b"172.16.1.27"),
        (b"id", b"0.0.0.0"),
        (b"sequence", b"0x80000004"),
        (b"age", b"106"),
        (b"checksum", b"0x274F"),
        (
            b"body",
            b"netmask=0.0.0.0\nforwarding-address=0.0.0.0\nmetric=1 type-1\nroute-tag=0\n",
        ),
        (b"dynamic", b"true"),
    ]);
    assert_eq!(external._type, OspfLsaType::External);
    assert_eq!(external.area, None);
    let link: RoutingOspfLsaState = build::<RoutingOspfLsaStateBuilder, _>(&[
        (b"instance", b"default-v3"),
        (b"area", b"backbone-v3"),
        (b"link", b"e15"),
        (b"type", b"link"),
        (b"originator", b"192.168.100.3"),
        (b"id", b"0.0.0.110"),
        (b"sequence", b"0x80000120"),
        (b"age", b"1388"),
        (b"checksum", b"0x238"),
        (
            b"body",
            b"options=V6|E|R\naddress=fe80::d601:c3ff:fe29:6772\npriority=128\n",
        ),
        (b"self-originated", b"true"),
        (b"dynamic", b"true"),
    ]);
    assert_eq!(link._type, OspfLsaType::Link);
    assert_eq!(link.self_originated, Some(true));
}

#[test]
fn decodes_a_health_row() {
    let row: SystemHealthState = build::<SystemHealthStateBuilder, _>(&[
        (b"name", b"psu1-voltage"),
        (b"value", b"24.2"),
        (b"type", b"V"),
    ]);
    assert_eq!(
        row.name.map(|name| name.to_string()),
        Some("psu1-voltage".into())
    );
}

#[test]
fn decodes_the_ospf_interfaces() {
    let common: [(&[u8], &[u8]); 13] = [
        (b"instance-id", b"0"),
        (b"type", b"broadcast"),
        (b"cost", b"100"),
        (b"priority", b"128"),
        (b"use-bfd", b"false"),
        (b"retransmit-interval", b"5s"),
        (b"transmit-delay", b"1s"),
        (b"hello-interval", b"10s"),
        (b"dead-interval", b"40s"),
        (b"dynamic", b"true"),
        (b"disabled", b"false"),
        (b"interface", b"e05-hex"),
        (b"state", b"bdr"),
    ];
    let v2: RoutingOspfInterfaceState = build::<RoutingOspfInterfaceStateBuilder, _>(
        &[
            &common[..],
            &[
                (b".id", b"*80000044"),
                (b"address", b"192.168.101.29%e05-hex"),
                (b"area", b"backbone-v2"),
                (b"dr", b"192.168.101.30"),
            ],
        ]
        .concat(),
    );
    assert_eq!(v2.interface.to_string(), "e05-hex");
    assert_eq!(v2.state, OspfInterfaceState::Bdr);
    assert_eq!(v2._type, OspfInterfaceType::Broadcast);
    assert_eq!(v2.dr, Some("192.168.101.30".parse().unwrap()));
    assert_eq!(v2.bdr, None);
    assert_eq!(v2.dead_interval, std::time::Duration::from_secs(40));
    // v3 without an address
    let v3: RoutingOspfInterfaceState = build::<RoutingOspfInterfaceStateBuilder, _>(
        &[
            &common[..],
            &[
                (b".id", b"*80000045"),
                (b"area", b"backbone-v3"),
                (b"dr", b"192.168.100.9"),
            ],
        ]
        .concat(),
    );
    assert_eq!(v3.address, None);
    let other: RoutingOspfInterfaceState = build::<RoutingOspfInterfaceStateBuilder, _>(
        &[
            &common[..12],
            &[
                (b".id", b"*1"),
                (b"area", b"backbone-v2"),
                (b"state", b"dr-other"),
            ],
        ]
        .concat(),
    );
    assert_eq!(other.state, OspfInterfaceState::DrOther);
}

#[test]
fn decodes_the_ipv4_address_list() {
    // a dynamic entry with the time it has left, and a network
    let added: IpFirewallAddressList = build::<IpFirewallAddressListBuilder, _>(&[
        (b".id", b"*1"),
        (b"list", b"probe"),
        (b"address", b"192.168.77.9"),
        (b"creation-time", b"2026-11-13 08:41:20"),
        (b"timeout", b"59s"),
        (b"dynamic", b"true"),
        (b"disabled", b"false"),
    ]);
    assert_eq!(added.cfg.address.to_string(), "192.168.77.9");
    assert_eq!(added.cfg.timeout, Some(std::time::Duration::from_secs(59)));
    assert!(added.status.dynamic);
    let network: IpFirewallAddressList = build::<IpFirewallAddressListBuilder, _>(&[
        (b".id", b"*2"),
        (b"list", b"probe"),
        (b"address", b"192.168.77.0/29"),
        (b"creation-time", b"2026-11-13 08:41:20"),
        (b"dynamic", b"false"),
        (b"disabled", b"false"),
    ]);
    assert_eq!(network.cfg.list.to_string(), "probe");
    assert_eq!(network.cfg.timeout, None);
}

#[test]
fn decodes_a_disabled_dhcp_client() {
    // without `invalid`, as CHR 7.24.5 reports a disabled client
    let client: IpDhcpClient = build::<IpDhcpClientBuilder, _>(&[
        (b".id", b"*2"),
        (b"interface", b"probe-br"),
        (b"add-default-route", b"yes"),
        (b"default-route-distance", b"1"),
        (b"use-peer-dns", b"true"),
        (b"use-peer-ntp", b"true"),
        (b"dhcp-options", b"hostname,clientid"),
        (b"status", b"stopped"),
        (b"dynamic", b"false"),
        (b"disabled", b"true"),
    ]);
    assert!(client.status.disabled);
    assert_eq!(client.status.invalid, None);
}
