//! Static decode tests for `/interface/wireless` (`ros_model/interface.txt`) and
//! `/interface/w60g` (`ros_model/interface/W60g.txt`).
//!
//! The values are the `print detail` output of the production radio bridges
//! (`bachmann-hegglin-2`/`hegglin-bachmann`, RBDynaDishG-5HacDr3, 5 GHz, and
//! `schopf-hegglin`/`hegglin-schopf`, RBwAPG-60ad-A, 60 GHz), one AP and one station
//! side each, with booleans in the API's `true`/`false` form. CHR has neither the
//! wireless nor the w60g menu, so this is the only check of these entities short of a
//! real device. Keys are placeholders.

use crate::ascii::AsciiString;
use crate::model::{
    InterfaceW60G, InterfaceW60GBuilder, InterfaceW60GMode, InterfaceWireless,
    InterfaceWirelessBuilder, InterfaceWirelessMode, InterfaceWirelessWirelessProtocol,
};
use crate::resource::{DeserializeRosBuilder, DeserializeRosResource};
use crate::value::Auto;

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

/// `print detail` of a 5 GHz `wlan1`, with the values that differ between the two
/// sides of the link passed in.
fn wireless_fields<'a>(
    mode: &'a [u8],
    channel_width: &'a [u8],
    wireless_protocol: &'a [u8],
    wds_mode: &'a [u8],
    wds_default_bridge: &'a [u8],
) -> Vec<(&'a [u8], &'a [u8])> {
    vec![
        (b"default-name", b"wlan1"),
        (b"name", b"wlan1"),
        (b"mtu", b"1500"),
        (b"l2mtu", b"1600"),
        (b"mac-address", b"64:D1:54:61:D9:2D"),
        (b"arp", b"enabled"),
        (b"interface-type", b"Atheros AR9888"),
        (b"mode", mode),
        (b"ssid", b"hegglin-bachmann"),
        (b"frequency", b"auto"),
        (b"band", b"5ghz-onlyac"),
        (b"channel-width", channel_width),
        (b"secondary-frequency", b""),
        (b"scan-list", b"default"),
        (b"wireless-protocol", wireless_protocol),
        (b"vlan-mode", b"no-tag"),
        (b"vlan-id", b"1"),
        (b"wds-mode", wds_mode),
        (b"wds-default-bridge", wds_default_bridge),
        (b"wds-ignore-ssid", b"no"),
        (b"bridge-mode", b"enabled"),
        (b"default-authentication", b"true"),
        (b"default-forwarding", b"true"),
        (b"default-ap-tx-limit", b"0"),
        (b"default-client-tx-limit", b"0"),
        (b"hide-ssid", b"false"),
        (b"security-profile", b"default"),
        (b"compression", b"false"),
        (b"disabled", b"false"),
        (b"running", b"true"),
    ]
}

#[test]
fn decodes_real_5ghz_station() {
    // bachmann-hegglin-2
    let wlan: InterfaceWireless = build::<InterfaceWirelessBuilder, _>(&wireless_fields(
        b"station-bridge",
        b"20/40/80mhz-XXXX",
        b"nv2-nstreme-802.11",
        b"disabled",
        b"none",
    ));
    assert_eq!(wlan.cfg.mode, Some(InterfaceWirelessMode::StationBridge));
    assert_eq!(
        wlan.cfg.wireless_protocol,
        Some(InterfaceWirelessWirelessProtocol::Nv2Nstreme80211)
    );
    assert_eq!(wlan.cfg.frequency, Some(Auto::Auto));
    assert_eq!(
        wlan.cfg.security_profile,
        Some(AsciiString::from("default"))
    );
}

#[test]
fn decodes_real_5ghz_ap() {
    // hegglin-bachmann: the AP side runs plain nv2 with dynamic WDS into `bridge`
    let wlan: InterfaceWireless = build::<InterfaceWirelessBuilder, _>(&wireless_fields(
        b"bridge",
        b"20/40/80mhz-Ceee",
        b"nv2",
        b"dynamic",
        b"bridge",
    ));
    assert_eq!(wlan.cfg.mode, Some(InterfaceWirelessMode::Bridge));
    assert_eq!(
        wlan.cfg.wireless_protocol,
        Some(InterfaceWirelessWirelessProtocol::Nv2)
    );
}

/// `print detail` of a 60 GHz `wlan60-1`.
fn w60g_fields<'a>(
    mode: &'a [u8],
    isolate_stations: &'a [u8],
    password: Option<&'a [u8]>,
) -> Vec<(&'a [u8], &'a [u8])> {
    let mut fields: Vec<(&[u8], &[u8])> = vec![
        (b".id", b"*1"),
        (b"default-name", b"wlan60-1"),
        (b"name", b"wlan60-1"),
        (b"mtu", b"1500"),
        (b"l2mtu", b"1600"),
        (b"mac-address", b"18:FD:74:F5:43:A3"),
        (b"arp", b"enabled"),
        (b"arp-timeout", b"auto"),
        (b"region", b"eu"),
        (b"mode", mode),
        (b"ssid", b"hegglin-schopf"),
        (b"frequency", b"auto"),
        (b"default-scan-list", b"58320,60480,62640,64800"),
        (b"tx-sector", b"auto"),
        (b"put-stations-in-bridge", b"bridge"),
        (b"isolate-stations", isolate_stations),
        (b"disabled", b"false"),
        (b"running", b"true"),
    ];
    fields.extend(password.map(|p| (b"password".as_ref(), p)));
    fields
}

#[test]
fn decodes_real_60ghz_station() {
    // schopf-hegglin
    let wlan: InterfaceW60G = build::<InterfaceW60GBuilder, _>(&w60g_fields(
        b"station-bridge",
        b"false",
        Some(b"XXXXXXXX"),
    ));
    assert_eq!(wlan.cfg.mode, Some(InterfaceW60GMode::StationBridge));
    assert_eq!(
        wlan.cfg.put_stations_in_bridge,
        Some(AsciiString::from("bridge"))
    );
    assert_eq!(wlan.cfg.frequency, Some(Auto::Auto));
}

#[test]
fn decodes_real_60ghz_ap() {
    // hegglin-schopf: its stations are separate dynamic interfaces
    // (`/interface/w60g/station`), not modeled here
    let wlan: InterfaceW60G =
        build::<InterfaceW60GBuilder, _>(&w60g_fields(b"bridge", b"true", None));
    assert_eq!(wlan.cfg.mode, Some(InterfaceW60GMode::Bridge));
    assert_eq!(wlan.cfg.isolate_stations, Some(true));
}

// `print advanced detail`: the binary API returns every field, the advanced ones
// included, so these must all decode. `link-5-ap` (SXT G-5HPacD) is close to the
// factory settings, `hegglin-bachmann` (RBDynaDishG-5HacDr3) is tuned. Booleans in
// the API's `true`/`false` form, secrets replaced.
const LINK_5_AP: &[(&[u8], &[u8])] = &[
    (b"name", b"wlan1"),
    (b"mtu", b"1500"),
    (b"l2mtu", b"1600"),
    (b"mac-address", b"4C:5E:0C:D7:A7:D1"),
    (b"arp", b"enabled"),
    (b"disable-running-check", b"false"),
    (b"interface-type", b"Atheros AR9888"),
    (b"radio-name", b"4C5E0CD7A7D1"),
    (b"mode", b"ap-bridge"),
    (b"ssid", b"link-5"),
    (b"area", b""),
    (b"frequency-mode", b"regulatory-domain"),
    (b"country", b"switzerland"),
    (b"installation", b"outdoor"),
    (b"antenna-gain", b"14"),
    (b"frequency", b"auto"),
    (b"band", b"5ghz-onlyac"),
    (b"channel-width", b"20/40mhz-XX"),
    (b"secondary-frequency", b""),
    (b"scan-list", b"default"),
    (b"wireless-protocol", b"any"),
    (b"rate-set", b"default"),
    (b"supported-rates-a/g", b"6Mbps,9Mbps,12Mbps,18Mbps,24Mbps,36Mbps,48Mbps,54Mbps"),
    (b"basic-rates-a/g", b"6Mbps"),
    (b"max-station-count", b"2007"),
    (b"distance", b"dynamic"),
    (b"tx-power-mode", b"default"),
    (b"vlan-mode", b"no-tag"),
    (b"vlan-id", b"1"),
    (b"wds-mode", b"disabled"),
    (b"wds-default-bridge", b"none"),
    (b"wds-default-cost", b"100"),
    (b"wds-cost-range", b"50-150"),
    (b"wds-ignore-ssid", b"false"),
    (b"update-stats-interval", b"disabled"),
    (b"bridge-mode", b"enabled"),
    (b"default-authentication", b"true"),
    (b"default-forwarding", b"true"),
    (b"default-ap-tx-limit", b"0"),
    (b"default-client-tx-limit", b"0"),
    (b"wmm-support", b"disabled"),
    (b"hide-ssid", b"false"),
    (b"security-profile", b"default"),
    (b"wps-mode", b"push-button"),
    (b"station-roaming", b"disabled"),
    (b"disconnect-timeout", b"3s"),
    (b"on-fail-retry-time", b"100ms"),
    (b"preamble-mode", b"both"),
    (b"compression", b"false"),
    (b"allow-sharedkey", b"false"),
    (b"station-bridge-clone-mac", b"00:00:00:00:00:00"),
    (b"ampdu-priorities", b"0"),
    (b"guard-interval", b"any"),
    (b"ht-supported-mcs", b"mcs-0,mcs-1,mcs-2,mcs-3,mcs-4,mcs-5,mcs-6,mcs-7,mcs-8,mcs-9,mcs-10,mcs-11,mcs-12,mcs-13,mcs-14,mcs-15,mcs-16,mcs-17,mcs-18,mcs-19,mcs-20,mcs-21,mcs-22,mcs-23"),
    (b"ht-basic-mcs", b"mcs-0,mcs-1,mcs-2,mcs-3,mcs-4,mcs-5,mcs-6,mcs-7"),
    (b"vht-supported-mcs", b"mcs0-9,mcs0-9,mcs0-9"),
    (b"vht-basic-mcs", b"mcs0-7"),
    (b"tx-chains", b"0,1"),
    (b"rx-chains", b"0,1"),
    (b"amsdu-limit", b"8192"),
    (b"amsdu-threshold", b"8192"),
    (b"tdma-period-size", b"2"),
    (b"nv2-queue-count", b"2"),
    (b"nv2-qos", b"default"),
    (b"nv2-cell-radius", b"30"),
    (b"nv2-security", b"disabled"),
    (b"nv2-mode", b"dynamic-downlink"),
    (b"nv2-downlink-ratio", b"50"),
    (b"nv2-sync-secret", b""),
    (b"hw-retries", b"7"),
    (b"frame-lifetime", b"0"),
    (b"adaptive-noise-immunity", b"none"),
    (b"hw-fragmentation-threshold", b"disabled"),
    (b"hw-protection-mode", b"none"),
    (b"hw-protection-threshold", b"0"),
    (b"frequency-offset", b"0"),
    (b"rate-selection", b"advanced"),
    (b"multicast-helper", b"default"),
    (b"multicast-buffering", b"enabled"),
    (b"keepalive-frames", b"enabled"),
    (b"skip-dfs-channels", b"disabled"),
];

const HEGGLIN_BACHMANN: &[(&[u8], &[u8])] = &[
    (b"name", b"wlan1"),
    (b"mtu", b"1500"),
    (b"l2mtu", b"1600"),
    (b"mac-address", b"E4:8D:8C:9D:6A:0C"),
    (b"arp", b"enabled"),
    (b"disable-running-check", b"false"),
    (b"interface-type", b"Atheros AR9888"),
    (b"radio-name", b"E48D8C9D6A0C"),
    (b"mode", b"bridge"),
    (b"ssid", b"hegglin-bachmann"),
    (b"area", b""),
    (b"frequency-mode", b"manual-txpower"),
    (b"country", b"etsi 5.5-5.7 outdoor"),
    (b"installation", b"outdoor"),
    (b"antenna-gain", b"25"),
    (b"frequency", b"auto"),
    (b"band", b"5ghz-onlyac"),
    (b"channel-width", b"20/40/80mhz-Ceee"),
    (b"secondary-frequency", b""),
    (b"scan-list", b"default"),
    (b"wireless-protocol", b"nv2"),
    (b"rate-set", b"default"),
    (b"supported-rates-a/g", b"6Mbps,9Mbps,12Mbps,18Mbps,24Mbps,36Mbps,48Mbps,54Mbps"),
    (b"basic-rates-a/g", b"6Mbps"),
    (b"max-station-count", b"2007"),
    (b"distance", b"2"),
    (b"tx-power-mode", b"default"),
    (b"vlan-mode", b"no-tag"),
    (b"vlan-id", b"1"),
    (b"wds-mode", b"dynamic"),
    (b"wds-default-bridge", b"bridge"),
    (b"wds-default-cost", b"100"),
    (b"wds-cost-range", b"50-150"),
    (b"wds-ignore-ssid", b"false"),
    (b"update-stats-interval", b"10s"),
    (b"bridge-mode", b"enabled"),
    (b"default-authentication", b"true"),
    (b"default-forwarding", b"true"),
    (b"default-ap-tx-limit", b"0"),
    (b"default-client-tx-limit", b"0"),
    (b"wmm-support", b"disabled"),
    (b"hide-ssid", b"false"),
    (b"security-profile", b"default"),
    (b"wps-mode", b"disabled"),
    (b"station-roaming", b"enabled"),
    (b"disconnect-timeout", b"3s"),
    (b"on-fail-retry-time", b"100ms"),
    (b"preamble-mode", b"short"),
    (b"compression", b"false"),
    (b"allow-sharedkey", b"false"),
    (b"station-bridge-clone-mac", b"00:00:00:00:00:00"),
    (b"ampdu-priorities", b"0"),
    (b"guard-interval", b"any"),
    (b"ht-supported-mcs", b"mcs-0,mcs-1,mcs-2,mcs-3,mcs-4,mcs-5,mcs-6,mcs-7,mcs-8,mcs-9,mcs-10,mcs-11,mcs-12,mcs-13,mcs-14,mcs-15,mcs-16,mcs-17,mcs-18,mcs-19,mcs-20,mcs-21,mcs-22,mcs-23"),
    (b"ht-basic-mcs", b"mcs-0,mcs-1,mcs-2,mcs-3,mcs-4,mcs-5,mcs-6,mcs-7"),
    (b"vht-supported-mcs", b"mcs0-9,mcs0-9,mcs0-9"),
    (b"vht-basic-mcs", b"mcs0-7"),
    (b"tx-chains", b"0,1"),
    (b"rx-chains", b"0,1"),
    (b"amsdu-limit", b"8192"),
    (b"amsdu-threshold", b"8192"),
    (b"tdma-period-size", b"auto"),
    (b"nv2-queue-count", b"2"),
    (b"nv2-qos", b"default"),
    (b"nv2-cell-radius", b"10"),
    (b"nv2-security", b"enabled"),
    (b"nv2-preshared-key", b"placeholder-key"),
    (b"nv2-mode", b"dynamic-downlink"),
    (b"nv2-downlink-ratio", b"50"),
    (b"nv2-sync-secret", b"placeholder-secret"),
    (b"hw-retries", b"7"),
    (b"frame-lifetime", b"0"),
    (b"adaptive-noise-immunity", b"none"),
    (b"hw-fragmentation-threshold", b"disabled"),
    (b"hw-protection-mode", b"none"),
    (b"hw-protection-threshold", b"0"),
    (b"frequency-offset", b"0"),
    (b"rate-selection", b"advanced"),
    (b"multicast-helper", b"default"),
    (b"multicast-buffering", b"enabled"),
    (b"keepalive-frames", b"enabled"),
    (b"skip-dfs-channels", b"disabled"),
];

fn with_status<'a>(fields: &[(&'a [u8], &'a [u8])]) -> Vec<(&'a [u8], &'a [u8])> {
    let mut fields = fields.to_vec();
    fields.extend([
        (b"default-name".as_ref(), b"wlan1".as_ref()),
        (b"disabled", b"false"),
        (b"running", b"true"),
    ]);
    fields
}

#[test]
fn decodes_all_fields_of_a_5ghz_radio() {
    for fields in [LINK_5_AP, HEGGLIN_BACHMANN] {
        let _: InterfaceWireless = build::<InterfaceWirelessBuilder, _>(&with_status(fields));
    }
}
