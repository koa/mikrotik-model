// The types are generated; a derive would have to come from the generator, these defaults from
// RouterOS.
#![allow(clippy::derivable_impls)]
use super::*;
use crate::value::Auto;
use crate::{
    value,
    value::{HasDisabled, HasNone},
};
use std::net::Ipv4Addr;

impl Default for SystemIdentityCfg {
    fn default() -> Self {
        SystemIdentityCfg {
            name: b"GeneratedName".into(),
        }
    }
}
/// RouterOS's factory setting: IPv6 on.
impl Default for Ipv6SettingsCfg {
    fn default() -> Self {
        Ipv6SettingsCfg {
            disable_ipv_6: false,
        }
    }
}
/// RouterOS's factory setting: the resolver answers only the device itself, and asks only
/// the servers it learns (from a DHCP client).
impl Default for IpDnsCfg {
    fn default() -> Self {
        IpDnsCfg {
            allow_remote_requests: false,
            servers: Default::default(),
        }
    }
}
/// RouterOS's factory setting: off, SOCKS4 without authentication.
impl Default for IpSocksCfg {
    fn default() -> Self {
        IpSocksCfg {
            enabled: false,
            port: 1080,
            connection_idle_timeout: std::time::Duration::from_secs(120),
            max_connections: 200,
            vrf: b"main".into(),
            version: 4,
            auth_method: IpSocksAuthMethod::None,
        }
    }
}
/// RouterOS's factory setting (CHR 7.24.5): the hardware watchdog on, no address watched.
impl Default for SystemWatchdogCfg {
    fn default() -> Self {
        SystemWatchdogCfg {
            watchdog_timer: true,
            automatic_supout: true,
            auto_send_supout: false,
            watch_address: HasNone::NoneValue,
            ping_start_after_boot: std::time::Duration::from_secs(5 * 60),
            ping_timeout: std::time::Duration::from_secs(60),
        }
    }
}
/// RouterOS's factory setting: no SSH forwarding; the rest is the device's own (`k`): host
/// keys and ciphers aren't managed, and an update switches password login on for a moment
/// (`always-allow-password-login` up to 7.20, `password-authentication` on 7.24).
impl Default for IpSshCfg {
    fn default() -> Self {
        IpSshCfg {
            forwarding_enabled: IpSshForwardingEnabled::No,
            always_allow_password_login: None,
            password_authentication: None,
            publickey_authentication_options: None,
            strong_crypto: None,
            host_key_size: None,
            host_key_type: None,
            ciphers: None,
        }
    }
}
impl Default for SystemRouterboardSettingsCfg {
    fn default() -> Self {
        SystemRouterboardSettingsCfg {
            auto_upgrade: true,
            baud_rate: None,
            boot_delay: None,
            boot_device: SystemRouterboardSettingsBootDevice::NandIfFailThenEthernet,
            boot_os: None,
            boot_protocol: SystemRouterboardSettingsBootProtocol::Bootp,
            cpu_frequency: None,
            cpu_mode: None,
            enable_jumper_reset: None,
            enter_setup_on: None,
            force_backup_booter: false,
            init_delay: None,
            memory_frequency: None,
            memory_data_rate: None,
            preboot_etherboot: value::HasDisabled::Disabled,
            preboot_etherboot_server: b"any".into(),
            regulatory_domain_ce: None,
            silent_boot: false,
            protected_routerboot: EnabledDisabled::Disabled,
            reformat_hold_button: Duration::from_secs(20),
            reformat_hold_button_max: Duration::from_secs(10 * 60),
            disable_pci: None,
        }
    }
}
#[allow(clippy::derivable_impls)]
impl Default for InterfaceWifiRadioSettingsCfg {
    fn default() -> Self {
        InterfaceWifiRadioSettingsCfg {
            external_antenna: None,
            wifi_band: None,
        }
    }
}
impl Default for InterfaceWifiCapsmanCfg {
    fn default() -> Self {
        // RouterOS reports these only once they were set (also to their default), so they stay
        // the device's own (`k`) where CAPsMAN is off; a controller gets explicit values
        InterfaceWifiCapsmanCfg {
            ca_certificate: None,
            certificate: None,
            enabled: YesNo::No,
            interfaces: None,
            package_path: None,
            require_peer_certificate: None,
            upgrade_policy: None,
        }
    }
}

impl Default for InterfaceWifiCapCfg {
    fn default() -> Self {
        InterfaceWifiCapCfg {
            caps_man_addresses: Default::default(),
            caps_man_certificate_common_names: Default::default(),
            caps_man_names: Default::default(),
            certificate: Default::default(),
            discovery_interfaces: Default::default(),
            enabled: YesNo::No,
            lock_to_caps_man: None,
            slaves_datapath: None,
            slaves_static: None,
        }
    }
}
impl Default for InterfaceVxlanCfg {
    fn default() -> Self {
        InterfaceVxlanCfg {
            allow_fast_path: true,
            arp: InterfaceVxlanArp::Enabled,
            arp_timeout: Some(value::Auto::Auto),
            comment: Default::default(),
            disabled: false,
            dont_fragment: InterfaceVxlanDontFragment::Disabled,
            group: Default::default(),
            interface: Default::default(),
            local_address: Default::default(),
            loop_protect: InterfaceVxlanLoopProtect::Default,
            loop_protect_disable_time: Duration::from_secs(5 * 60),
            loop_protect_send_interval: Duration::from_secs(5),
            mac_address: None,
            max_fdb_size: 4096,
            mtu: 1500,
            name: Default::default(),
            port: 8472,
            vni: 1,
            vtep_vrf: Some(b"main".into()),
            vteps_ip_version: InterfaceVxlanVtepsIpVersion::Ipv4,
        }
    }
}

impl Default for InterfaceBridgeCfg {
    fn default() -> Self {
        Self {
            add_dhcp_option_82: None,
            admin_mac: None,
            ageing_time: Duration::from_secs(5 * 60),
            arp: InterfaceBridgeArp::Enabled,
            arp_timeout: value::Auto::Auto,
            auto_mac: true,
            comment: None,
            dhcp_snooping: false,
            disabled: false,
            ether_type: Some(InterfaceBridgeEtherType::_0X8100),
            fast_forward: true,
            forward_delay: Some(Duration::from_secs(15)),
            forward_reserved_addresses: None,
            frame_types: Some(VlanFrameTypes::AdmitAll),
            igmp_snooping: false,
            igmp_version: None,
            ingress_filtering: Some(true),
            last_member_interval: None,
            last_member_query_count: None,
            max_hops: Some(20),
            max_learned_entries: value::HasUnlimited::Value(value::Auto::Auto),
            max_message_age: Some(Duration::from_secs(20)),
            membership_interval: None,
            mld_version: None,
            mtu: value::Auto::Auto,
            multicast_querier: None,
            multicast_router: None,
            mvrp: None,
            name: Default::default(),
            port_cost_mode: InterfaceBridgePortCostMode::Long,
            priority: Some(value::Hex(0x8000)),
            protocol_mode: InterfaceBridgeProtocolMode::Rstp,
            pvid: Some(1),
            querier_interval: None,
            query_interval: None,
            query_response_interval: None,
            region_name: None,
            region_revision: Some(0),
            startup_query_count: None,
            startup_query_interval: None,
            transmit_hold_count: Some(6),
            vlan_filtering: false,
            mac_address: None,
        }
    }
}

impl Default for InterfaceBridgeByName {
    fn default() -> Self {
        Self(InterfaceBridgeCfg::default())
    }
}

impl Default for InterfaceBridgePortCfg {
    fn default() -> Self {
        Self {
            broadcast_flood: true,
            edge: InterfaceBridgePortEdge::Auto,
            interface: Default::default(),
            bridge: Default::default(),
            multicast_router: InterfaceBridgePortMulticastRouter::TemporaryQuery,
            priority: value::Hex(128),
            restricted_tcn: false,
            unknown_multicast_flood: true,
            comment: None,
            fast_leave: false,
            tag_stacking: false,
            unknown_unicast_flood: true,
            frame_types: VlanFrameTypes::AdmitAll,
            ingress_filtering: true,
            learn: value::Auto::Auto,
            horizon: Some(HasNone::NoneValue),
            point_to_point: value::Auto::Auto,
            restricted_role: false,
            trusted: false,
            disabled: false,
            bpdu_guard: false,
            auto_isolate: false,
            pvid: 1,
            hw: Some(true),
        }
    }
}
impl Default for InterfaceBridgeVlanCfg {
    fn default() -> Self {
        Self {
            bridge: Default::default(),
            comment: None,
            disabled: false,
            tagged: Default::default(),
            untagged: Default::default(),
            vlan_ids: Default::default(),
            mvrp_forbidden: Default::default(),
        }
    }
}
impl Default for InterfaceVlanCfg {
    fn default() -> Self {
        InterfaceVlanCfg {
            arp: InterfaceVlanArp::Enabled,
            arp_timeout: value::Auto::Auto,
            comment: None,
            disabled: false,
            interface: Default::default(),
            loop_protect: InterfaceVlanLoopProtect::Default,
            loop_protect_disable_time: Duration::from_secs(5 * 60),
            loop_protect_send_interval: Duration::from_secs(5),
            mac_address: None,
            mtu: Some(1500),
            name: Default::default(),
            use_service_tag: false,
            vlan_id: 0,
            mvrp: Some(false),
        }
    }
}
impl Default for InterfaceVxlanVtepsCfg {
    fn default() -> Self {
        InterfaceVxlanVtepsCfg {
            disabled: false,
            comment: None,
            interface: Default::default(),
            remote_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
        }
    }
}
#[allow(clippy::derivable_impls)]
impl Default for InterfaceWirelessCapCfg {
    fn default() -> Self {
        InterfaceWirelessCapCfg {
            bridge: HasNone::NoneValue,
            caps_man_addresses: Default::default(),
            caps_man_certificate_common_names: Default::default(),
            caps_man_names: None,
            certificate: HasNone::NoneValue,
            discovery_interfaces: Default::default(),
            enabled: false,
            interfaces: Default::default(),
            lock_to_caps_man: false,
            static_virtual: false,
        }
    }
}
impl Default for CapsManAaaCfg {
    fn default() -> Self {
        CapsManAaaCfg {
            called_format: CapsManAaaCalledFormat::MacSsid,
            interim_update: HasDisabled::Disabled,
            mac_caching: HasDisabled::Disabled,
            mac_format: b"XX:XX:XX:XX:XX:XX".into(),
            mac_mode: CapsManAaaMacMode::AsUsername,
        }
    }
}
impl Default for CapsManManagerCfg {
    fn default() -> Self {
        CapsManManagerCfg {
            ca_certificate: HasNone::NoneValue,
            certificate: HasNone::NoneValue,
            enabled: false,
            package_path: Default::default(),
            require_peer_certificate: false,
            upgrade_policy: HasNone::NoneValue,
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for IpAddressCfg {
    fn default() -> Self {
        IpAddressCfg {
            address: Default::default(),
            interface: Default::default(),
            comment: None,
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for Ipv6AddressCfg {
    fn default() -> Self {
        Ipv6AddressCfg {
            address: Default::default(),
            advertise: false,
            interface: Default::default(),
            comment: None,
            disabled: false,
            eui_64: false,
            auto_link_local: true,
            from_pool: None,
            no_dad: false,
        }
    }
}
impl Default for SystemPackageLocalUpdateMirrorCfg {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(60 * 60 * 24),
            enabled: false,
            password: Default::default(),
            primary_server: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            secondary_server: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            user: Default::default(),
        }
    }
}
impl Default for SystemPackageUpdateCfg {
    fn default() -> Self {
        Self {
            channel: SystemPackageUpdateChannel::Stable,
        }
    }
}
impl Default for RoutingOspfInstanceCfg {
    fn default() -> Self {
        Self {
            comment: None,
            disabled: false,
            domain_id: None,
            domain_tag: None,
            in_filter: None,
            mpls_te_address: None,
            mpls_te_area: None,
            name: Default::default(),
            originate_default: Some(RoutingOriginateDefault::IfInstalled),
            out_filter_chain: None,
            out_filter_select: None,
            redistribute: Default::default(),
            router_id: None,
            version: RoutingOspfInstanceVersion::_2,
            vrf: b"main".into(),
            use_dn: None,
            in_filter_chain: None,
            routing_table: None,
        }
    }
}
impl Default for RoutingOspfAreaCfg {
    fn default() -> Self {
        Self {
            area_id: Ipv4Addr::new(0, 0, 0, 0),
            comment: None,
            default_cost: None,
            disabled: false,
            instance: Default::default(),
            name: Default::default(),
            nssa_translator: None,
            _type: RoutingOspfAreaType::Default,
        }
    }
}
impl Default for RoutingOspfAreaByName {
    fn default() -> Self {
        Self(RoutingOspfAreaCfg::default())
    }
}
impl Default for RoutingOspfInterfaceTemplateCfg {
    fn default() -> Self {
        Self {
            area: Default::default(),
            auth: None,
            auth_id: None,
            auth_key: None,
            comment: None,
            cost: 100,
            dead_interval: Duration::from_secs(40),
            disabled: false,
            hello_interval: Duration::from_secs(10),
            instance_id: 0,
            interfaces: Default::default(),
            networks: Default::default(),
            prefix_list: Default::default(),
            priority: 128,
            retransmit_interval: Duration::from_secs(5),
            transmit_delay: Duration::from_secs(1),
            _type: RoutingOspfInterfaceTemplateType::Broadcast,
            use_bfd: None,
            vlink_neighbor_id: None,
            vlink_transit_area: None,
        }
    }
}
#[allow(clippy::derivable_impls)]
impl Default for IpDhcpServerConfigCfg {
    fn default() -> Self {
        IpDhcpServerConfigCfg {
            accounting: false,
            interim_update: Default::default(),
            radius_password: Default::default(),
            store_leases_disk: Default::default(),
        }
    }
}
impl Default for IpDhcpServerCfg {
    /// RouterOS's factory settings. Most of them it reports only when changed (7.20.8 and
    /// 7.24.5 alike), so they are `None` here: a value would be a diff forever.
    fn default() -> Self {
        IpDhcpServerCfg {
            add_arp: None,
            address_lists: Default::default(),
            address_pool: b"static-only".into(),
            allow_dual_stack_queue: None,
            always_broadcast: None,
            authoritative: None,
            bootp_lease_time: None,
            bootp_support: None,
            client_mac_limit: None,
            comment: Default::default(),
            conflict_detection: None,
            delay_threshold: None,
            dhcp_option_set: None,
            disabled: false,
            insert_queue_before: None,
            interface: Default::default(),
            lease_script: Default::default(),
            lease_time: Some(b"30m".into()),
            name: Default::default(),
            parent_queue: None,
            relay: Default::default(),
            server_address: None,
            use_framed_as_classless: None,
            use_radius: IpDhcpServerUseRadius::No,
        }
    }
}
#[allow(clippy::derivable_impls)]
impl Default for IpDhcpServerByName {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[allow(clippy::derivable_impls)]
impl Default for IpPoolCfg {
    fn default() -> Self {
        IpPoolCfg {
            comment: Default::default(),
            name: Default::default(),
            next_pool: None,
            ranges: Default::default(),
        }
    }
}
#[allow(clippy::derivable_impls)]
impl Default for IpPoolByName {
    fn default() -> Self {
        IpPoolByName(IpPoolCfg::default())
    }
}
#[allow(clippy::derivable_impls)]
impl Default for IpDhcpServerNetworkCfg {
    fn default() -> Self {
        IpDhcpServerNetworkCfg {
            address: Default::default(),
            boot_file_name: Default::default(),
            caps_manager: None,
            comment: Default::default(),
            dhcp_option: Default::default(),
            dhcp_option_set: None,
            // RouterOS reports it only when set
            dns_none: false,
            dns_server: Default::default(),
            domain: Default::default(),
            gateway: Default::default(),
            netmask: 0,
            next_server: Default::default(),
            ntp_server: Default::default(),
            wins_server: Default::default(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for IpDhcpServerNetworkByAddress {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[allow(clippy::derivable_impls)]
impl Default for InterfaceWifiDatapathByName {
    fn default() -> Self {
        InterfaceWifiDatapathByName(Default::default())
    }
}
#[allow(clippy::derivable_impls)]
impl Default for InterfaceWifiDatapathCfg {
    fn default() -> Self {
        InterfaceWifiDatapathCfg {
            bridge: None,
            bridge_cost: None,
            bridge_horizon: None,
            client_isolation: None,
            comment: None,
            disabled: false,
            interface_list: None,
            name: Default::default(),
            vlan_id: None,
        }
    }
}
#[allow(clippy::derivable_impls)]
impl Default for InterfaceWifiConfigurationByName {
    fn default() -> Self {
        InterfaceWifiConfigurationByName(Default::default())
    }
}
#[allow(clippy::derivable_impls)]
impl Default for InterfaceWifiConfigurationCfg {
    fn default() -> Self {
        InterfaceWifiConfigurationCfg {
            aaa: None,
            aaa_called_format: None,
            aaa_calling_format: None,
            aaa_interim_update: None,
            aaa_mac_caching: None,
            aaa_nas_identifier: None,
            aaa_password_format: None,
            aaa_username_format: None,
            antenna_gain: None,
            beacon_interval: None,
            chains: Default::default(),
            channel: None,
            channel_band: None,
            channel_frequency: Default::default(),
            channel_reselect_interval: None,
            channel_secondary_frequency: None,
            channel_skip_dfs_channels: None,
            channel_width: None,
            comment: None,
            country: None,
            datapath: None,
            datapath_bridge: None,
            datapath_bridge_cost: None,
            datapath_bridge_horizon: None,
            datapath_client_isolation: None,
            datapath_interface_list: None,
            datapath_vlan_id: None,
            disabled: false,
            distance: None,
            dtim_period: None,
            hide_ssid: None,
            installation: None,
            interworking: None,
            interworking_3_gpp_info: Default::default(),
            interworking_authentication_types: Default::default(),
            interworking_connection_capabilities: Default::default(),
            interworking_domain_names: Default::default(),
            interworking_esr: None,
            interworking_hessid: None,
            interworking_hotspot_20: None,
            interworking_hotspot_20_dgaf: None,
            interworking_internet: None,
            interworking_ipv_4_availability: None,
            interworking_ipv_6_availability: None,
            interworking_network_type: None,
            interworking_operational_classes: Default::default(),
            interworking_operator_names: Default::default(),
            interworking_realms: Default::default(),
            interworking_roaming_ois: Default::default(),
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
            manager: None,
            mode: None,
            multicast_enhance: None,
            name: Default::default(),
            qos_classifier: None,
            security: None,
            security_authentication_types: Default::default(),
            security_connect_group: None,
            security_connect_priority: None,
            security_dh_groups: Default::default(),
            security_disable_pmkid: None,
            security_eap_accounting: None,
            security_eap_anonymous_identity: None,
            security_eap_certificate_mode: None,
            security_eap_methods: Default::default(),
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
            ssid: None,
            station_roaming: None,
            steering: None,
            steering_neighbor_group: None,
            steering_rrm: None,
            steering_wnm: None,
            tx_chains: Default::default(),
            tx_power: None,
        }
    }
}

impl Default for InterfaceWifiProvisioningCfg {
    fn default() -> Self {
        InterfaceWifiProvisioningCfg {
            action: InterfaceWifiProvisioningAction::CreateDisabled,
            address_ranges: Default::default(),
            comment: None,
            common_name_regexp: None,
            disabled: false,
            identity_regexp: None,
            master_configuration: None,
            name_format: None,
            radio_mac: None,
            slave_configurations: None,
            slave_name_format: None,
            supported_bands: Default::default(),
        }
    }
}
#[allow(clippy::derivable_impls)]
impl Default for CapsManSecurityByName {
    fn default() -> Self {
        CapsManSecurityByName(Default::default())
    }
}
#[allow(clippy::derivable_impls)]
impl Default for CapsManSecurityCfg {
    fn default() -> Self {
        CapsManSecurityCfg {
            authentication_types: Default::default(),
            comment: None,
            disable_pmkid: None,
            eap_methods: None,
            eap_radius_accounting: None,
            encryption: Default::default(),
            group_encryption: None,
            group_key_update: None,
            name: Default::default(),
            passphrase: None,
            tls_certificate: None,
            tls_mode: None,
        }
    }
}
#[allow(clippy::derivable_impls)]
impl Default for CapsManDatapathByName {
    fn default() -> Self {
        CapsManDatapathByName(Default::default())
    }
}
#[allow(clippy::derivable_impls)]
impl Default for CapsManDatapathCfg {
    fn default() -> Self {
        CapsManDatapathCfg {
            arp: None,
            bridge: None,
            bridge_cost: None,
            bridge_horizon: None,
            client_to_client_forwarding: None,
            comment: None,
            interface_list: None,
            l_2_mtu: None,
            local_forwarding: None,
            mtu: None,
            name: Default::default(),
            openflow_switch: None,
            vlan_id: None,
            vlan_mode: None,
        }
    }
}
#[allow(clippy::derivable_impls)]
impl Default for CapsManConfigurationByName {
    fn default() -> Self {
        CapsManConfigurationByName(Default::default())
    }
}
#[allow(clippy::derivable_impls)]
impl Default for CapsManConfigurationCfg {
    fn default() -> Self {
        CapsManConfigurationCfg {
            channel: None,
            channel_band: None,
            channel_control_channel_width: None,
            channel_extension_channel: None,
            channel_frequency: None,
            channel_reselect_interval: None,
            channel_save_selected: None,
            channel_secondary_frequency: None,
            channel_skip_dfs_channels: None,
            channel_tx_power: None,
            comment: None,
            country: None,
            datapath: None,
            datapath_arp: None,
            datapath_bridge: None,
            datapath_bridge_cost: None,
            datapath_bridge_horizon: None,
            datapath_client_to_client_forwarding: None,
            datapath_interface_list: None,
            datapath_l_2_mtu: None,
            datapath_local_forwarding: None,
            datapath_mtu: None,
            datapath_openflow_switch: None,
            datapath_vlan_id: None,
            datapath_vlan_mode: None,
            disconnect_timeout: None,
            distance: None,
            frame_lifetime: None,
            guard_interval: None,
            hide_ssid: None,
            hw_protection_mode: None,
            hw_retries: None,
            installation: None,
            keepalive_frames: None,
            load_balancing_group: None,
            max_sta_count: None,
            mode: None,
            multicast_helper: None,
            name: Default::default(),
            rates: None,
            rates_basic: Default::default(),
            rates_ht_basic_mcs: Default::default(),
            rates_ht_supported_mcs: Default::default(),
            rates_supported: Default::default(),
            rates_vht_basic_mcs: Default::default(),
            rates_vht_supported_mcs: Default::default(),
            rx_chains: Default::default(),
            security: None,
            security_authentication_types: Default::default(),
            security_disable_pmkid: None,
            security_eap_methods: None,
            security_eap_radius_accounting: None,
            security_encryption: Default::default(),
            security_group_encryption: None,
            security_group_key_update: None,
            security_passphrase: None,
            security_tls_certificate: None,
            security_tls_mode: None,
            ssid: None,
            tx_chains: Default::default(),
        }
    }
}
/// A rule that matches every radio (RouterOS reports the empty `radio-mac` as zeros).
impl Default for CapsManProvisioningCfg {
    fn default() -> Self {
        CapsManProvisioningCfg {
            action: HasNone::NoneValue,
            comment: None,
            common_name_regexp: Default::default(),
            disabled: false,
            hw_supported_modes: Default::default(),
            identity_regexp: Default::default(),
            ip_address_ranges: Default::default(),
            master_configuration: Default::default(),
            name_format: CapsManProvisioningNameFormat::Cap,
            name_prefix: Default::default(),
            radio_mac: b"00:00:00:00:00:00".into(),
            slave_configurations: Default::default(),
        }
    }
}
impl Default for SystemHealthSettingsCfg {
    fn default() -> Self {
        SystemHealthSettingsCfg {
            cpu_overtemp_check: false,
            cpu_overtemp_startup_delay: Default::default(),
            cpu_overtemp_threshold: 0,
            fan_control_interval: Default::default(),
            fan_full_speed_temp: 0,
            fan_min_speed_percent: 0,
            fan_mode: Auto::Auto,
            fan_on_threshold: 0,
            fan_switch: Auto::Auto,
            fan_target_temp: 0,
            use_fan: SystemHealthSettingsUseFan::Auxiliary,
            use_fan_2: SystemHealthSettingsUseFan2::Auxiliary,
        }
    }
}
/// RouterOS's setting without default configuration: mac-telnet on every interface (the default
/// configuration of a RouterBOARD narrows it to its list `LAN`).
impl Default for ToolMacServerCfg {
    fn default() -> Self {
        ToolMacServerCfg {
            allowed_interface_list: b"all".into(),
        }
    }
}

/// RouterOS 7 without the default configuration discovers on every static interface (the
/// default configuration sets `LAN`).
impl Default for IpNeighborDiscoverySettingsCfg {
    fn default() -> Self {
        IpNeighborDiscoverySettingsCfg {
            discover_interface_list: b"static".into(),
        }
    }
}
