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
}
