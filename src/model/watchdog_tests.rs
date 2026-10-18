//! Static decode tests for `/system/watchdog` (`ros_model/system/Watchdog.txt`), with the API
//! output of CHR 7.24.5.

use crate::model::{SystemWatchdogCfg, SystemWatchdogCfgBuilder};
use crate::resource::{DeserializeRosBuilder, DeserializeRosResource};
use crate::value::HasNone;
use std::net::{IpAddr, Ipv4Addr};

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
fn decodes_the_factory_settings() {
    let watchdog: SystemWatchdogCfg = build::<SystemWatchdogCfgBuilder, _>(&[
        (b"watch-address", b"none"),
        (b"watchdog-timer", b"true"),
        (b"ping-start-after-boot", b"5m"),
        (b"ping-timeout", b"1m"),
        (b"automatic-supout", b"true"),
        (b"auto-send-supout", b"false"),
    ]);
    assert_eq!(watchdog, SystemWatchdogCfg::default());
}

#[test]
fn decodes_a_watch_address() {
    let watchdog: SystemWatchdogCfg = build::<SystemWatchdogCfgBuilder, _>(&[
        (b"watch-address", b"192.168.100.2"),
        (b"watchdog-timer", b"true"),
        (b"ping-start-after-boot", b"15m"),
        (b"ping-timeout", b"10m"),
        (b"automatic-supout", b"true"),
        (b"auto-send-supout", b"false"),
    ]);
    assert_eq!(
        watchdog.watch_address,
        HasNone::Value(IpAddr::V4(Ipv4Addr::new(192, 168, 100, 2)))
    );
}
