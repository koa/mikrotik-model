include!(concat!(env!("OUT_DIR"), "/mikrotik-model.rs"));

#[cfg(test)]
mod bgp_tests;
#[cfg(test)]
mod bridge_tests;
mod defaults;
#[cfg(test)]
mod dhcp_tests;
mod enums;
#[cfg(test)]
mod reference_tests;
#[cfg(test)]
mod firewall_tests;
#[cfg(test)]
mod ospf_tests;
#[cfg(test)]
mod socks_tests;
#[cfg(test)]
mod watchdog_tests;
#[cfg(test)]
mod wireless_tests;
