//! Static decode tests for the menus the provisioner only reads (`file.txt`, `log.txt`,
//! `system/Scheduler.txt`, `system/Script.txt`, `system/DeviceMode.txt`, `user.txt`,
//! `ipv6/Route.txt`, `ip/ip.txt`'s `/ip/route`), with the API output of CHR 7.24.5. A field the
//! model doesn't know fails the test. `/partitions`, `/ip/neighbor` and `/user/ssh-keys` had
//! no entries on the CHR.

use crate::model::{
    File, FileBuilder, IpRoute, IpRouteBuilder, Ipv6RouteState, Ipv6RouteStateBuilder, LogState,
    LogStateBuilder, SystemDeviceModeState, SystemDeviceModeStateBuilder, SystemScheduler,
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
