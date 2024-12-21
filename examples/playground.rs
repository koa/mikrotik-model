use mikrotik_model::model::SystemArchitecture;
use mikrotik_model::value::RosValue;
use mikrotik_model::{resource, value};
use std::time::Duration;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum EthernetSpeed {
    _10MBaseTHalf,
    _10MBaseTFull,
    _100MBaseTHalf,
    _100MBaseTFull,
    _100MBaseFxHalf,
    _100MBaseFxFull,
    _1GBaseTHalf,
    _1GBaseTFull,
    _1GBaseX,
    _2_5GBaseT,
    _2_5GBaseX,
    _5GBaseT,
    _10GBaseT,
    _10GBaseCr,
    _10GBaseSrLr,
    _25GBaseCr,
    _25GBaseSrLr,
    _40GBaseCr4,
    _40GBaseSr4Lr4,
    _50GBaseCr,
    _50GBaseCr2,
    _50GBaseSrLr,
    _50GBaseSr2Lr2,
    _100GBaseCr2,
    _100GBaseSr2Lr2,
    _100GBaseCr4,
    _100GBaseSr4Lr4,
    _200GBaseCr4,
    _200GBaseSr4Lr4,
    _400GBaseCr8,
    _400GBaseSr8Lr8,
}
impl mikrotik_model::value::RosValue for EthernetSpeed {
    fn parse_ros(value: &str) -> mikrotik_model::value::ParseRosValueResult<Self> {
        match value {
            "10M-baseT-half" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_10MBaseTHalf)
            }
            "10M-baseT-full" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_10MBaseTFull)
            }
            "100M-baseT-half" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_100MBaseTHalf)
            }
            "100M-baseT-full" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_100MBaseTFull)
            }
            "100M-baseFX-half" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_100MBaseFxHalf)
            }
            "100M-baseFX-full" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_100MBaseFxFull)
            }
            "1G-baseT-half" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_1GBaseTHalf)
            }
            "1G-baseT-full" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_1GBaseTFull)
            }
            "1G-baseX" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_1GBaseX)
            }
            "2.5G-baseT" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_2_5GBaseT)
            }
            "2.5G-baseX" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_2_5GBaseX)
            }
            "5G-baseT" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_5GBaseT)
            }
            "10G-baseT" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_10GBaseT)
            }
            "10G-baseCR" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_10GBaseCr)
            }
            "10G-baseSR-LR" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_10GBaseSrLr)
            }
            "25G-baseCR" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_25GBaseCr)
            }
            "25G-baseSR-LR" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_25GBaseSrLr)
            }
            "40G-baseCR4" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_40GBaseCr4)
            }
            "40G-baseSR4-LR4" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_40GBaseSr4Lr4)
            }
            "50G-baseCR" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_50GBaseCr)
            }
            "50G-baseCR2" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_50GBaseCr2)
            }
            "50G-baseSR-LR" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_50GBaseSrLr)
            }
            "50G-baseSR2-LR2" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_50GBaseSr2Lr2)
            }
            "100G-baseCR2" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_100GBaseCr2)
            }
            "100G-baseSR2-LR2" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_100GBaseSr2Lr2)
            }
            "100G-baseCR4" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_100GBaseCr4)
            }
            "100G-baseSR4-LR4" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_100GBaseSr4Lr4)
            }
            "200G-baseCR4" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_200GBaseCr4)
            }
            "200G-baseSR4-LR4" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_200GBaseSr4Lr4)
            }
            "400G-baseCR8" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_400GBaseCr8)
            }
            "400G-baseSR8-LR8" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetSpeed::_400GBaseSr8Lr8)
            }
            &_ => mikrotik_model::value::ParseRosValueResult::Invalid,
        }
    }
    fn encode_ros(&self) -> std::borrow::Cow<str> {
        std::borrow::Cow::Borrowed(match self {
            EthernetSpeed::_10MBaseTHalf => "10M-baseT-half",
            EthernetSpeed::_10MBaseTFull => "10M-baseT-full",
            EthernetSpeed::_100MBaseTHalf => "100M-baseT-half",
            EthernetSpeed::_100MBaseTFull => "100M-baseT-full",
            EthernetSpeed::_100MBaseFxHalf => "100M-baseFX-half",
            EthernetSpeed::_100MBaseFxFull => "100M-baseFX-full",
            EthernetSpeed::_1GBaseTHalf => "1G-baseT-half",
            EthernetSpeed::_1GBaseTFull => "1G-baseT-full",
            EthernetSpeed::_1GBaseX => "1G-baseX",
            EthernetSpeed::_2_5GBaseT => "2.5G-baseT",
            EthernetSpeed::_2_5GBaseX => "2.5G-baseX",
            EthernetSpeed::_5GBaseT => "5G-baseT",
            EthernetSpeed::_10GBaseT => "10G-baseT",
            EthernetSpeed::_10GBaseCr => "10G-baseCR",
            EthernetSpeed::_10GBaseSrLr => "10G-baseSR-LR",
            EthernetSpeed::_25GBaseCr => "25G-baseCR",
            EthernetSpeed::_25GBaseSrLr => "25G-baseSR-LR",
            EthernetSpeed::_40GBaseCr4 => "40G-baseCR4",
            EthernetSpeed::_40GBaseSr4Lr4 => "40G-baseSR4-LR4",
            EthernetSpeed::_50GBaseCr => "50G-baseCR",
            EthernetSpeed::_50GBaseCr2 => "50G-baseCR2",
            EthernetSpeed::_50GBaseSrLr => "50G-baseSR-LR",
            EthernetSpeed::_50GBaseSr2Lr2 => "50G-baseSR2-LR2",
            EthernetSpeed::_100GBaseCr2 => "100G-baseCR2",
            EthernetSpeed::_100GBaseSr2Lr2 => "100G-baseSR2-LR2",
            EthernetSpeed::_100GBaseCr4 => "100G-baseCR4",
            EthernetSpeed::_100GBaseSr4Lr4 => "100G-baseSR4-LR4",
            EthernetSpeed::_200GBaseCr4 => "200G-baseCR4",
            EthernetSpeed::_200GBaseSr4Lr4 => "200G-baseSR4-LR4",
            EthernetSpeed::_400GBaseCr8 => "400G-baseCR8",
            EthernetSpeed::_400GBaseSr8Lr8 => "400G-baseSR8-LR8",
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum DhcpOption {
    Clientid,
    ClientidDuid,
    Hostname,
}
impl mikrotik_model::value::RosValue for DhcpOption {
    fn parse_ros(value: &str) -> mikrotik_model::value::ParseRosValueResult<Self> {
        match value {
            "clientid" => mikrotik_model::value::ParseRosValueResult::Value(DhcpOption::Clientid),
            "clientid_duid" => {
                mikrotik_model::value::ParseRosValueResult::Value(DhcpOption::ClientidDuid)
            }
            "hostname" => mikrotik_model::value::ParseRosValueResult::Value(DhcpOption::Hostname),
            &_ => mikrotik_model::value::ParseRosValueResult::Invalid,
        }
    }
    fn encode_ros(&self) -> std::borrow::Cow<str> {
        std::borrow::Cow::Borrowed(match self {
            DhcpOption::Clientid => "clientid",
            DhcpOption::ClientidDuid => "clientid_duid",
            DhcpOption::Hostname => "hostname",
        })
    }
}
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum EthernetArp {
    Disabled,
    Enabled,
    LocalProxyArp,
    ProxyArp,
    ReplyOnly,
}
impl mikrotik_model::value::RosValue for EthernetArp {
    fn parse_ros(value: &str) -> mikrotik_model::value::ParseRosValueResult<Self> {
        match value {
            "disabled" => mikrotik_model::value::ParseRosValueResult::Value(EthernetArp::Disabled),
            "enabled" => mikrotik_model::value::ParseRosValueResult::Value(EthernetArp::Enabled),
            "local-proxy-arp" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetArp::LocalProxyArp)
            }
            "proxy-arp" => mikrotik_model::value::ParseRosValueResult::Value(EthernetArp::ProxyArp),
            "reply-only" => {
                mikrotik_model::value::ParseRosValueResult::Value(EthernetArp::ReplyOnly)
            }
            &_ => mikrotik_model::value::ParseRosValueResult::Invalid,
        }
    }
    fn encode_ros(&self) -> std::borrow::Cow<str> {
        std::borrow::Cow::Borrowed(match self {
            EthernetArp::Disabled => "disabled",
            EthernetArp::Enabled => "enabled",
            EthernetArp::LocalProxyArp => "local-proxy-arp",
            EthernetArp::ProxyArp => "proxy-arp",
            EthernetArp::ReplyOnly => "reply-only",
        })
    }
}
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum BridgeMaxLearnedEntries {
    Auto,
    Unlimited,
    Value(u32),
}

impl mikrotik_model::value::RosValue for BridgeMaxLearnedEntries {
    fn parse_ros(value: &str) -> mikrotik_model::value::ParseRosValueResult<Self> {
        match value {
            "auto" => {
                mikrotik_model::value::ParseRosValueResult::Value(BridgeMaxLearnedEntries::Auto)
            }
            "unlimited" => mikrotik_model::value::ParseRosValueResult::Value(
                BridgeMaxLearnedEntries::Unlimited,
            ),
            value => u32::parse_ros(value).map(BridgeMaxLearnedEntries::Value),
        }
    }
    fn encode_ros(&self) -> std::borrow::Cow<str> {
        match self {
            BridgeMaxLearnedEntries::Auto => std::borrow::Cow::Borrowed("auto"),
            BridgeMaxLearnedEntries::Unlimited => std::borrow::Cow::Borrowed("unlimited"),
            BridgeMaxLearnedEntries::Value(v) => v.encode_ros(),
        }
    }
}

fn main() {
    let value = EthernetSpeed::parse_ros("10G-baseCR").ok();
    println!("{:?}, {}", value, value.unwrap().encode_ros());
}
