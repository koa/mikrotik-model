use super::*;
use crate::value;
use enum_iterator::{all, Sequence};
#[allow(clippy::derivable_impls)]
impl Default for SystemResourceCfg {
    fn default() -> Self {
        SystemResourceCfg {
            cpu_frequency: None,
        }
    }
}
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
            discovery_interfaces: value::HasNone::NoneValue,
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

struct DataResourceRefIterator<'a> {
    data: &'a Data,
    state: Option<ResourceType>,
}

impl<'a> Iterator for DataResourceRefIterator<'a> {
    type Item = Resource;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(resource_type) = self.state.as_ref() {
            resource_type.next();
        }
        todo!()
    }
}
