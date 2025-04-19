use super::*;
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
        InterfaceWifiCapsmanCfg {
            ca_certificate: None,
            certificate: None,
            enabled: YesNo::No,
            interfaces: Default::default(),
            package_path: Default::default(),
            require_peer_certificate: false,
            upgrade_policy: InterfaceWifiCapsmanUpgradePolicy::SuggestSameVersion,
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
            vrf: b"main".into(),
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
            mvrp: false,
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
            horizon: None,
            point_to_point: value::Auto::Auto,
            restricted_role: false,
            trusted: false,
            disabled: false,
            bpdu_guard: false,
            auto_isolate: false,
            pvid: 1,
            hw: None,
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
            l_2_mtu: 1556,
            loop_protect: InterfaceVlanLoopProtect::Default,
            loop_protect_disable_time: Duration::from_secs(5 * 60),
            loop_protect_send_interval: Duration::from_secs(5),
            loop_protect_status: InterfaceVlanLoopProtectStatus::Off,
            mac_address: None,
            mtu: 1500,
            name: Default::default(),
            use_service_tag: false,
            vlan_id: 0,
            mvrp: None,
        }
    }
}
impl Default for InterfaceVxlanVtepsCfg {
    fn default() -> Self {
        InterfaceVxlanVtepsCfg {
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
            bridge: None,
            caps_man_addresses: Default::default(),
            caps_man_certificate_common_names: Default::default(),
            caps_man_names: None,
            certificate: None,
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
