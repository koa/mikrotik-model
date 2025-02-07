use crate::ascii::AsciiString;
use crate::model::{
    Data, EthernetSpeed, InterfaceEthernetArp, InterfaceEthernetByDefaultName,
    InterfaceEthernetCfg, InterfaceEthernetLoopProtect, OnOff,
};
use crate::value::{Auto, HasUnlimited, RxTxPair};
use std::time::Duration;

#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub enum DeviceType {
    RB750Gr3,
    CRS32624G2Splus,
    CCR10097G1C1Splus,
}
impl DeviceType {
    pub fn ethernet_port_count(&self) -> usize {
        match self {
            DeviceType::RB750Gr3 => 5,
            DeviceType::CRS32624G2Splus => 24,
            DeviceType::CCR10097G1C1Splus => 7,
        }
    }
    pub fn combo_port_count(&self) -> usize {
        match self {
            DeviceType::RB750Gr3 => 0,
            DeviceType::CRS32624G2Splus => 0,
            DeviceType::CCR10097G1C1Splus => 1,
        }
    }
    pub fn sfp_sfpplus_port_count(&self) -> usize {
        match self {
            DeviceType::RB750Gr3 => 0,
            DeviceType::CRS32624G2Splus => 2,
            DeviceType::CCR10097G1C1Splus => 1,
        }
    }
    pub fn device_type_name(&self) -> &'static str {
        match self {
            DeviceType::RB750Gr3 => "RB750Gr3",
            DeviceType::CRS32624G2Splus => "CRS326-24G-2S+",
            DeviceType::CCR10097G1C1Splus => "CCR1009-7G-1C-1S+",
        }
    }
    pub fn generate_empty_data(&self) -> Data {
        let mut data = Data::default();
        for idx in 1..=self.ethernet_port_count() {
            let default_name = format!("ether{idx}");
            let short_name = format!("e{idx:02}");
            data.interface_ethernet_by_default_name
                .push(InterfaceEthernetByDefaultName {
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
                        cable_setting: None,
                        combo_mode: None,
                        comment: None,
                        disable_running_check: None,
                        fec_mode: None,
                        tx_flow_control: Some(Auto::Value(OnOff::Off)),
                        rx_flow_control: Some(Auto::Value(OnOff::Off)),
                        full_duplex: None,
                        l_2_mtu: 1596,
                        mac_address: None,
                        mdix_enable: None,
                        mtu: 1500,
                        name: AsciiString(Box::from(short_name.as_bytes())),
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
                });
        }
        data
    }
}
