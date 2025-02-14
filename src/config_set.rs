use crate::ascii::AsciiString;
use crate::hwconfig::DeviceType;
use crate::model::{
    EthernetSpeed, InterfaceEthernetArp, InterfaceEthernetByDefaultName,
    InterfaceEthernetCableSetting, InterfaceEthernetCfg, InterfaceEthernetLoopProtect, OnOff,
    Resource,
};
use crate::value::{Auto, HasUnlimited, RxTxPair};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ConfigSet {
    pub configs: Vec<Resource>,
}

impl ConfigSet {
    /*pub fn new(device_type: DeviceType) -> ConfigSet {
        let mut configs = Vec::new();
        for idx in 1..=device_type.ethernet_port_count() {
            let default_name = format!("ether{idx}");
            configs.push(Resource::InterfaceEthernetByDefaultName(
                InterfaceEthernetByDefaultName {
                    default_name: AsciiString(Box::from(default_name.as_bytes())),
                    data: InterfaceEthernetCfg {
                        advertise: [
                            EthernetSpeed::_10MBaseTHalf,
                            EthernetSpeed::_10MBaseTFull,
                            EthernetSpeed::_100MBaseTHalf,
                            EthernetSpeed::_100MBaseTFull,
                            EthernetSpeed::_1GBaseTHalf,
                            EthernetSpeed::_1GBaseTFull,
                        ]
                        .into(),
                        arp: InterfaceEthernetArp::Enabled,
                        arp_timeout: Some(Auto::Auto),
                        auto_negotiation: true,
                        bandwidth: RxTxPair {
                            rx: HasUnlimited::Unlimited,
                            tx: HasUnlimited::Unlimited,
                        },
                        cable_setting: Some(InterfaceEthernetCableSetting::Default),
                        combo_mode: None,
                        comment: None,
                        disable_running_check: Some(true),
                        fec_mode: None,
                        tx_flow_control: Some(Auto::Value(OnOff::Off)),
                        rx_flow_control: Some(Auto::Value(OnOff::Off)),
                        full_duplex: None,
                        l_2_mtu: 1596,
                        mac_address: None,
                        mdix_enable: None,
                        mtu: 1500,
                        name: AsciiString(Box::from(default_name.as_bytes())),
                        passthrough_interface: None,
                        poe_out: None,
                        poe_priority: None,
                        sfp_shutdown_temperature: None,
                        sfp_rate_select: None,
                        speed: None,
                        sfp_ignore_rx_los: None,
                        disabled: false,
                        loop_protect_disable_time: Duration::from_secs(5 * 60),
                        loop_protect_send_interval: Duration::from_secs(5),
                        loop_protect: InterfaceEthernetLoopProtect::Default,
                    },
                },
            ))
        }
        ConfigSet { configs }
    }*/
}
