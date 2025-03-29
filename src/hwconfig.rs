use crate::model::{InterfaceWirelessByDefaultName, InterfaceWirelessCfg};
use crate::{
    ascii::AsciiString,
    model::{
        Data, EthernetSpeed, InterfaceEthernetArp, InterfaceEthernetByDefaultName,
        InterfaceEthernetCfg, InterfaceEthernetComboMode, InterfaceEthernetFecMode,
        InterfaceEthernetLoopProtect, InterfaceEthernetPoeOut, InterfaceEthernetSfpRateSelect,
        InterfaceWifiByDefaultName, InterfaceWifiCfg, OnOff,
    },
    value::{Auto, HasUnlimited, RxTxPair},
};
use std::{iter::repeat_n, time::Duration};

#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub enum DeviceType {
    RB750Gr3,
    Crs1098g1s2hnD,
    Crs32624g2splus,
    Ccr10097g1c1splus,
    Crs35448g4splus2qplus,
    C52iG5haxD2haxD,
}
impl DeviceType {
    pub fn type_by_name(name: &[u8]) -> Option<DeviceType> {
        match name {
            b"RB750Gr3" => Some(DeviceType::RB750Gr3),
            b"CRS326-24G-2S+" => Some(DeviceType::Crs32624g2splus),
            b"CCR1009-7G-1C-1S+" => Some(DeviceType::Ccr10097g1c1splus),
            b"CRS354-48G-4S+2Q+" => Some(DeviceType::Crs35448g4splus2qplus),
            b"C52iG-5HaxD2HaxD" => Some(DeviceType::C52iG5haxD2haxD),
            b"CRS109-8G-1S-2HnD" => Some(DeviceType::Crs1098g1s2hnD),
            _ => None,
        }
    }
    pub fn device_type_name(&self) -> &'static str {
        match self {
            DeviceType::RB750Gr3 => "RB750Gr3",
            DeviceType::Crs32624g2splus => "CRS326-24G-2S+",
            DeviceType::Ccr10097g1c1splus => "CCR1009-7G-1C-1S+",
            DeviceType::Crs35448g4splus2qplus => "CRS354-48G-4S+2Q+",
            DeviceType::C52iG5haxD2haxD => "C52iG-5HaxD2HaxD",
            DeviceType::Crs1098g1s2hnD => "CRS109-8G-1S-2HnD",
        }
    }

    pub fn build_ethernet_ports(&self) -> Vec<InterfaceEthernetByDefaultName> {
        match self {
            DeviceType::RB750Gr3 => repeat_n(
                generate_ethernet(EthernetNamePattern::Ether, &ADVERTISE_1G, 1596, false),
                5,
            )
            .enumerate()
            .map(|(idx, generator)| generator(idx + 1))
            .collect(),
            DeviceType::C52iG5haxD2haxD => repeat_n(
                generate_ethernet(EthernetNamePattern::Ether, &ADVERTISE_1G, 1568, true),
                1,
            )
            .chain(repeat_n(
                generate_ethernet(EthernetNamePattern::Ether, &ADVERTISE_1G, 1568, false),
                4,
            ))
            .enumerate()
            .map(|(idx, generator)| generator(idx + 1))
            .collect(),
            DeviceType::Crs32624g2splus => repeat_n(
                generate_ethernet(EthernetNamePattern::Ether, &ADVERTISE_1G, 1592, false),
                24,
            )
            .enumerate()
            .chain(
                repeat_n(
                    generate_ethernet(EthernetNamePattern::SfpSfpPlus, &ADVERTISE_10G, 1592, false),
                    2,
                )
                .enumerate(),
            )
            .map(|(idx, generator)| generator(idx + 1))
            .collect(),
            DeviceType::Ccr10097g1c1splus => repeat_n(
                generate_ethernet(EthernetNamePattern::Ether, &ADVERTISE_1G_FULL, 1580, false),
                7,
            )
            .enumerate()
            .chain(
                repeat_n(
                    generate_ethernet(EthernetNamePattern::Combo, &ADVERTISE_1G_FULL, 1580, false),
                    1,
                )
                .enumerate(),
            )
            .chain(
                repeat_n(
                    generate_ethernet(
                        EthernetNamePattern::SfpSfpPlus,
                        &ADVERTISE_10G_FULL,
                        1580,
                        false,
                    ),
                    1,
                )
                .enumerate(),
            )
            .map(|(idx, generator)| generator(idx + 1))
            .collect(),
            DeviceType::Crs35448g4splus2qplus => repeat_n(
                generate_ethernet(EthernetNamePattern::Ether, &ADVERTISE_1G, 1592, false),
                48,
            )
            .chain(repeat_n(
                generate_ethernet(EthernetNamePattern::Ether, &ADVERTISE_100M, 1592, false),
                1,
            ))
            .enumerate()
            .chain(
                repeat_n(
                    generate_ethernet(EthernetNamePattern::SfpSfpPlus, &ADVERTISE_10G, 1592, false),
                    4,
                )
                .enumerate(),
            )
            .chain(
                repeat_n(
                    generate_ethernet(EthernetNamePattern::QsfpPlus, &ADVERTISE_10G, 1592, false),
                    4 * 2,
                )
                .enumerate(),
            )
            .map(|(idx, generator)| generator(idx + 1))
            .collect(),
            DeviceType::Crs1098g1s2hnD => repeat_n(
                generate_ethernet(EthernetNamePattern::Ether, &ADVERTISE_1G, 1588, false),
                8,
            )
            .enumerate()
            .chain(
                repeat_n(
                    generate_ethernet(EthernetNamePattern::Sfp, &ADVERTISE_1G_SFP, 1588, false),
                    1,
                )
                .enumerate(),
            )
            .map(|(idx, generator)| generator(idx + 1))
            .collect(),
        }
    }
    pub fn build_wifi_ports(&self) -> Vec<InterfaceWifiByDefaultName> {
        match self {
            DeviceType::C52iG5haxD2haxD => repeat_n(generate_wifi(1560), 2)
                .enumerate()
                .map(|(idx, generator)| generator(idx + 1))
                .collect(),
            DeviceType::Crs32624g2splus
            | DeviceType::Crs1098g1s2hnD
            | DeviceType::Ccr10097g1c1splus
            | DeviceType::RB750Gr3
            | DeviceType::Crs35448g4splus2qplus => Vec::new(),
        }
    }
    pub fn build_wireless_ports(&self) -> Vec<InterfaceWirelessByDefaultName> {
        match self {
            DeviceType::RB750Gr3
            | DeviceType::Crs32624g2splus
            | DeviceType::Ccr10097g1c1splus
            | DeviceType::Crs35448g4splus2qplus
            | DeviceType::C52iG5haxD2haxD => Default::default(),
            DeviceType::Crs1098g1s2hnD => repeat_n(generate_wlan(1600), 1)
                .enumerate()
                .map(|(idx, generator)| generator(idx + 1))
                .collect(),
        }
    }

    pub fn generate_empty_data(&self) -> Data {
        Data {
            interface_ethernet_by_default_name: self.build_ethernet_ports(),
            interface_wifi_by_default_name: self.build_wifi_ports(),
            ..Data::default()
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Hash)]
enum EthernetNamePattern {
    Ether,
    Combo,
    Sfp,
    SfpSfpPlus,
    QsfpPlus,
}
impl EthernetNamePattern {
    fn default_name(&self, idx: usize) -> AsciiString {
        AsciiString(Box::from(
            (match self {
                EthernetNamePattern::Ether => {
                    format!("ether{idx}")
                }
                EthernetNamePattern::SfpSfpPlus => {
                    format!("sfp-sfpplus{idx}")
                }
                EthernetNamePattern::QsfpPlus => {
                    format!("qsfpplus{}-{}", ((idx - 1) / 4) + 1, ((idx - 1) % 4) + 1)
                }
                EthernetNamePattern::Combo => {
                    format!("combo{idx}")
                }
                EthernetNamePattern::Sfp => {
                    format!("sfp{idx}")
                }
            })
            .as_bytes(),
        ))
    }
    fn short_name(&self, idx: usize) -> AsciiString {
        AsciiString(Box::from(
            (match self {
                EthernetNamePattern::Ether => {
                    format!("e{idx:02}")
                }
                EthernetNamePattern::SfpSfpPlus => {
                    format!("s{idx:02}")
                }
                EthernetNamePattern::QsfpPlus => {
                    format!("q{:02}-{}", ((idx - 1) / 4) + 1, ((idx - 1) % 4) + 1)
                }
                EthernetNamePattern::Combo => {
                    format!("c{idx:02}")
                }
                EthernetNamePattern::Sfp => {
                    format!("s{idx:02}")
                }
            })
            .as_bytes(),
        ))
    }
    fn default_combo_mode(&self) -> Option<InterfaceEthernetComboMode> {
        if let EthernetNamePattern::Combo = self {
            Some(InterfaceEthernetComboMode::Auto)
        } else {
            None
        }
    }
    fn default_sfp_shutdown_temperature(&self) -> Option<u8> {
        match self {
            EthernetNamePattern::Ether => None,
            EthernetNamePattern::Combo
            | EthernetNamePattern::SfpSfpPlus
            | EthernetNamePattern::QsfpPlus
            | EthernetNamePattern::Sfp => Some(95),
        }
    }
    fn default_sfp_ignore_rx_loss(&self) -> Option<bool> {
        match self {
            EthernetNamePattern::Ether => None,
            EthernetNamePattern::Combo => Some(false),
            EthernetNamePattern::SfpSfpPlus => Some(false),
            EthernetNamePattern::QsfpPlus => Some(false),
            EthernetNamePattern::Sfp => Some(false),
        }
    }
    fn default_sfp_rate_select(&self) -> Option<InterfaceEthernetSfpRateSelect> {
        match self {
            EthernetNamePattern::Ether => None,
            EthernetNamePattern::Combo => Some(InterfaceEthernetSfpRateSelect::High),
            EthernetNamePattern::SfpSfpPlus => Some(InterfaceEthernetSfpRateSelect::High),
            EthernetNamePattern::QsfpPlus => Some(InterfaceEthernetSfpRateSelect::High),
            EthernetNamePattern::Sfp => Some(InterfaceEthernetSfpRateSelect::High),
        }
    }
    fn default_fec_mode(&self) -> Option<InterfaceEthernetFecMode> {
        match self {
            EthernetNamePattern::Ether => None,
            EthernetNamePattern::Combo => None,

            EthernetNamePattern::SfpSfpPlus => None,
            EthernetNamePattern::QsfpPlus => Some(InterfaceEthernetFecMode::Auto),
            EthernetNamePattern::Sfp => None,
        }
    }
}

fn generate_ethernet(
    name: EthernetNamePattern,
    speeds: &[EthernetSpeed],
    l_2_mtu: u16,
    has_poe_out: bool,
) -> impl Fn(usize) -> InterfaceEthernetByDefaultName + Clone + use<'_> {
    move |idx| InterfaceEthernetByDefaultName {
        default_name: name.default_name(idx),
        data: InterfaceEthernetCfg {
            advertise: speeds.iter().cloned().collect(),
            arp: InterfaceEthernetArp::Enabled,
            arp_timeout: Some(Auto::Auto),
            auto_negotiation: true,
            bandwidth: RxTxPair {
                rx: HasUnlimited::Unlimited,
                tx: HasUnlimited::Unlimited,
            },
            cable_setting: None,
            combo_mode: name.default_combo_mode(),
            comment: None,
            disable_running_check: None,
            fec_mode: name.default_fec_mode(),
            tx_flow_control: Some(Auto::Value(OnOff::Off)),
            rx_flow_control: Some(Auto::Value(OnOff::Off)),
            full_duplex: None,
            l_2_mtu,
            mac_address: None,
            mdix_enable: None,
            mtu: 1500,
            name: name.short_name(idx),
            passthrough_interface: None,
            poe_out: if has_poe_out {
                Some(InterfaceEthernetPoeOut::AutoOn)
            } else {
                None
            },
            poe_priority: if has_poe_out { Some(10) } else { None },
            sfp_shutdown_temperature: name.default_sfp_shutdown_temperature(),
            sfp_rate_select: name.default_sfp_rate_select(),
            speed: None,
            sfp_ignore_rx_los: name.default_sfp_ignore_rx_loss(),
            disabled: false,
            loop_protect_disable_time: Duration::from_secs(5 * 60),
            loop_protect_send_interval: Duration::from_secs(5),
            loop_protect: InterfaceEthernetLoopProtect::Default,
        },
    }
}

fn generate_wifi(l_2_mtu: u16) -> impl Fn(usize) -> InterfaceWifiByDefaultName + Clone {
    move |idx| {
        let default_name: AsciiString = format!("wifi{idx}").into();
        let name: AsciiString = format!("wi{idx:02}").into();
        InterfaceWifiByDefaultName {
            default_name: Some(default_name),
            data: InterfaceWifiCfg {
                aaa: None,
                aaa_called_format: None,
                aaa_calling_format: None,
                aaa_interim_update: None,
                aaa_mac_caching: None,
                aaa_nas_identifier: None,
                aaa_password_format: None,
                aaa_username_format: None,
                arp: None,
                arp_timeout: Some(Auto::Auto),
                channel: None,
                channel_band: None,
                channel_frequency: None,
                channel_reselect_interval: None,
                channel_secondary_frequency: None,
                channel_skip_dfs_channels: None,
                channel_width: None,
                comment: None,
                configuration: None,
                configuration_antenna_gain: None,
                configuration_beacon_interval: None,
                configuration_chains: Default::default(),
                configuration_country: None,
                configuration_distance: None,
                configuration_dtim_period: None,
                configuration_hide_ssid: None,
                configuration_installation: None,
                configuration_manager: None,
                configuration_mode: None,
                configuration_multicast_enhance: None,
                configuration_qos_classifier: None,
                configuration_ssid: None,
                configuration_station_roaming: None,
                configuration_tx_chains: Default::default(),
                configuration_tx_power: None,
                datapath: None,
                datapath_bridge: None,
                datapath_bridge_cost: None,
                datapath_bridge_horizon: None,
                datapath_client_isolation: None,
                datapath_interface_list: None,
                datapath_vlan_id: None,
                disable_running_check: None,
                disabled: false,
                interworking: None,
                interworking_3_gpp_info: None,
                interworking_authentication_types: None,
                interworking_connection_capabilities: None,
                interworking_domain_names: None,
                interworking_esr: None,
                interworking_hessid: None,
                interworking_hotspot_20: None,
                interworking_hotspot_20_dgaf: None,
                interworking_internet: None,
                interworking_ipv_4_availability: None,
                interworking_ipv_6_availability: None,
                interworking_network_type: None,
                interworking_operational_classes: None,
                interworking_operator_names: None,
                interworking_realms: None,
                interworking_roaming_ois: None,
                interworking_uesa: None,
                interworking_venue: None,
                interworking_venue_names: Default::default(),
                interworking_wan_at_capacity: None,
                interworking_wan_downlink: None,
                interworking_wan_downlink_load: None,
                interworking_wan_measurement_duration: None,
                interworking_wan_status: None,
                interworking_wan_symmetric: None,
                interworking_wan_uplink: None,
                interworking_wan_uplink_load: None,
                l_2_mtu,
                mac_address: None,
                master_interface: None,
                mtu: None,
                name,
                radio_mac: None,
                security: None,
                security_authentication_types: Default::default(),
                security_connect_group: None,
                security_connect_priority: None,
                security_dh_groups: None,
                security_disable_pmkid: None,
                security_eap_accounting: None,
                security_eap_anonymous_identity: None,
                security_eap_certificate_mode: None,
                security_eap_methods: None,
                security_eap_password: None,
                security_eap_tls_certificate: None,
                security_eap_username: None,
                security_encryption: None,
                security_ft: None,
                security_ft_mobility_domain: None,
                security_ft_nas_identifier: None,
                security_ft_over_ds: None,
                security_ft_preserve_vlanid: None,
                security_ft_r_0_key_lifetime: None,
                security_ft_reassociation_deadline: None,
                security_group_encryption: None,
                security_group_key_update: None,
                security_management_encryption: None,
                security_management_protection: None,
                security_multi_passphrase_group: None,
                security_owe_transition_interface: None,
                security_passphrase: None,
                security_sae_anti_clogging_threshold: None,
                security_sae_max_failure_rate: None,
                security_sae_pwe: None,
                security_wps: None,
                steering: None,
                steering_neighbor_group: None,
                steering_rrm: None,
                steering_wnm: None,
            },
        }
    }
}
fn generate_wlan(l_2_mtu: u16) -> impl Fn(usize) -> InterfaceWirelessByDefaultName + Clone {
    move |idx| {
        let default_name: AsciiString = format!("wlan{idx}").into();
        let name: AsciiString = format!("wl{idx:02}").into();
        InterfaceWirelessByDefaultName {
            default_name: Some(default_name),
            data: InterfaceWirelessCfg {
                adaptive_noise_immunity: None,
                allow_sharedkey: None,
                ampdu_priorities: None,
                amsdu_limit: None,
                amsdu_threshold: None,
                antenna_gain: None,
                antenna_mode: None,
                area: None,
                arp: None,
                arp_timeout: None,
                band: None,
                basic_rates_a_g: None,
                basic_rates_b: None,
                bridge_mode: None,
                burst_time: None,
                channel_width: None,
                comment: None,
                compression: None,
                country: None,
                default_ap_tx_limit: None,
                default_authentication: None,
                default_client_tx_limit: None,
                default_forwarding: None,
                disable_running_check: None,
                disabled: false,
                disconnect_timeout: None,
                distance: None,
                frame_lifetime: None,
                frequency: None,
                frequency_mode: None,
                frequency_offset: None,
                guard_interval: None,
                hide_ssid: None,
                ht_basic_mcs: Default::default(),
                ht_supported_mcs: Default::default(),
                hw_fragmentation_threshold: None,
                hw_protection_mode: None,
                hw_protection_threshold: None,
                hw_retries: None,
                installation: None,
                interworking_profile: None,
                keepalive_frames: None,
                l_2_mtu: Some(l_2_mtu),
                mac_address: None,
                master_interface: None,
                max_station_count: None,
                mode: None,
                mtu: None,
                multicast_buffering: None,
                multicast_helper: None,
                name,
                noise_floor_threshold: None,
                nv_2_cell_radius: None,
                nv_2_downlink_ratio: None,
                nv_2_mode: None,
                nv_2_noise_floor_offset: None,
                nv_2_preshared_key: None,
                nv_2_qos: None,
                nv_2_queue_count: None,
                nv_2_security: None,
                nv_2_sync_secret: None,
                on_fail_retry_time: None,
                preamble_mode: None,
                prism_cardtype: None,
                radio_name: None,
                rate_selection: None,
                rate_set: None,
                rx_chains: None,
                scan_list: None,
                secondary_frequency: None,
                security_profile: None,
                skip_dfs_channels: None,
                ssid: None,
                station_bridge_clone_mac: None,
                station_roaming: None,
                supported_rates_a_g: Default::default(),
                supported_rates_b: Default::default(),
                tdma_period_size: None,
                tx_chains: Default::default(),
                tx_power: None,
                tx_power_mode: None,
                update_stats_interval: None,
                vht_basic_mcs: None,
                vht_supported_mcs: None,
                vlan_id: None,
                vlan_mode: None,
                wds_cost_range: None,
                wds_default_bridge: None,
                wds_default_cost: None,
                wds_ignore_ssid: None,
                wds_mode: None,
                wireless_protocol: None,
                wmm_support: None,
                wps_mode: None,
            },
        }
    }
}

const ADVERTISE_1G: [EthernetSpeed; 6] = [
    EthernetSpeed::_10MBaseTHalf,
    EthernetSpeed::_10MBaseTFull,
    EthernetSpeed::_100MBaseTHalf,
    EthernetSpeed::_100MBaseTFull,
    EthernetSpeed::_1GBaseTHalf,
    EthernetSpeed::_1GBaseTFull,
];
const ADVERTISE_1G_SFP: [EthernetSpeed; 7] = [
    EthernetSpeed::_10MBaseTHalf,
    EthernetSpeed::_10MBaseTFull,
    EthernetSpeed::_100MBaseTHalf,
    EthernetSpeed::_100MBaseTFull,
    EthernetSpeed::_1GBaseTHalf,
    EthernetSpeed::_1GBaseTFull,
    EthernetSpeed::_1GBaseX,
];
const ADVERTISE_1G_FULL: [EthernetSpeed; 3] = [
    EthernetSpeed::_10MBaseTFull,
    EthernetSpeed::_100MBaseTFull,
    EthernetSpeed::_1GBaseTFull,
];

const ADVERTISE_100M: [EthernetSpeed; 4] = [
    EthernetSpeed::_10MBaseTHalf,
    EthernetSpeed::_10MBaseTFull,
    EthernetSpeed::_100MBaseTHalf,
    EthernetSpeed::_100MBaseTFull,
];

const ADVERTISE_10G: [EthernetSpeed; 13] = [
    EthernetSpeed::_10MBaseTHalf,
    EthernetSpeed::_10MBaseTFull,
    EthernetSpeed::_100MBaseTHalf,
    EthernetSpeed::_100MBaseTFull,
    EthernetSpeed::_1GBaseTHalf,
    EthernetSpeed::_1GBaseTFull,
    EthernetSpeed::_1GBaseX,
    EthernetSpeed::_25GBaseT,
    EthernetSpeed::_25GBaseX,
    EthernetSpeed::_5GBaseT,
    EthernetSpeed::_10GBaseT,
    EthernetSpeed::_10GBaseCr,
    EthernetSpeed::_10GBaseSrLr,
];
const ADVERTISE_10G_FULL: [EthernetSpeed; 7] = [
    EthernetSpeed::_10MBaseTFull,
    EthernetSpeed::_100MBaseTFull,
    EthernetSpeed::_1GBaseTFull,
    EthernetSpeed::_1GBaseX,
    EthernetSpeed::_10GBaseT,
    EthernetSpeed::_10GBaseCr,
    EthernetSpeed::_10GBaseSrLr,
];
