use config::{Config, Environment, File};
use const_str::ip_addr;
use encoding_rs::mem::decode_latin1;
use env_logger::{Env, TimestampPrecision};
use itertools::Itertools;
use log::{error, info};
use mikrotik_model::{
    Credentials, MikrotikDevice,
    ascii::AsciiString,
    generator::Generator,
    hwconfig::DeviceType,
    model::{
        InterfaceBridgeByName, InterfaceBridgeCfg, InterfaceBridgePortById, InterfaceBridgePortCfg,
        InterfaceBridgeProtocolMode, InterfaceBridgeVlanCfg, InterfaceEthernetByDefaultName,
        InterfaceVlanByName, InterfaceVlanCfg, InterfaceVxlanByName, InterfaceVxlanVtepsById,
        InterfaceWifiByDefaultName, InterfaceWifiCapCfg, InterfaceWifiDatapathByName,
        SystemIdentityCfg, SystemRouterboardSettingsCfg, VlanFrameTypes, YesNo,
    },
    model::{InterfaceVlan, InterfaceVxlanCfg, InterfaceVxlanVtepsCfg, ReferenceType},
    resource::{
        self, KeyedResource, ResourceMutation, ResourceMutationError, SingleResource,
        generate_add_update_remove_by_id, generate_add_update_remove_by_key,
        generate_single_update, generate_update_by_key,
    },
    value::PossibleRangeDash,
};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet},
    fmt::{Display, Formatter},
    net::Ipv6Addr,
    net::{IpAddr, Ipv4Addr},
};

struct DeviceDataCurrent {
    identity: SystemIdentityCfg,
    routerboard_settings: SystemRouterboardSettingsCfg,
    ethernet: Box<[InterfaceEthernetByDefaultName]>,
    wifi: Box<[InterfaceWifiByDefaultName]>,
    wifi_cap: InterfaceWifiCapCfg,
    wifi_datapath: Vec<InterfaceWifiDatapathByName>,
    bridge: Vec<InterfaceBridgeByName>,
    bridge_port: Vec<InterfaceBridgePortById>,
    vlans: Vec<InterfaceVlanByName>,
    vxlan: Vec<InterfaceVxlanByName>,
    vxlan_vteps: Vec<InterfaceVxlanVtepsById>,
}
struct DeviceDataTarget {
    identity: SystemIdentityCfg,
    routerboard_settings: SystemRouterboardSettingsCfg,
    ethernet: Box<[InterfaceEthernetByDefaultName]>,
    wifi: Box<[InterfaceWifiByDefaultName]>,
    wifi_cap: InterfaceWifiCapCfg,
    wifi_datapath: Vec<InterfaceWifiDatapathByName>,
    bridge: BTreeMap<AsciiString, InterfaceBridgeCfg>,
    bridge_port: BTreeMap<(AsciiString, AsciiString), InterfaceBridgePortCfg>,
    bridge_vlans: Vec<InterfaceBridgeVlanCfg>,
    vlans: BTreeMap<AsciiString, InterfaceVlanCfg>,
    vxlan: BTreeMap<AsciiString, InterfaceVxlanCfg>,
    vxlan_vteps: BTreeMap<(AsciiString, IpAddr), InterfaceVxlanVtepsCfg>,
}

impl DeviceDataTarget {
    fn new(device_type: DeviceType) -> Self {
        Self {
            identity: SystemIdentityCfg::default(),
            routerboard_settings: Default::default(),
            ethernet: device_type.build_ethernet_ports().into(),
            wifi: device_type.build_wifi_ports().into(),
            wifi_cap: InterfaceWifiCapCfg::default(),
            wifi_datapath: vec![],
            bridge: BTreeMap::default(),
            bridge_port: BTreeMap::default(),
            bridge_vlans: vec![],
            vlans: BTreeMap::default(),
            vxlan: BTreeMap::default(),
            vxlan_vteps: BTreeMap::default(),
        }
    }

    fn set_identity(&mut self, name: impl Into<AsciiString>) {
        self.identity.name = name.into();
    }

    fn enable_ospf(&mut self, loopback_v4: &Ipv4Addr, loopback_v6: &Ipv6Addr) {}

    fn enable_vxlan<'a>(
        &mut self,
        ifname: &[u8],
        vni: u32,
        other_vxlan: impl IntoIterator<Item = &'a IpAddr>,
    ) {
        let vxlan_if = self.vxlan.entry(ifname.into()).or_default();
        vxlan_if.vni = vni;
        for addr in other_vxlan {
            self.vxlan_vteps
                .entry((ifname.into(), addr.clone()))
                .or_default();
        }
    }

    fn enable_wifi_cap(&mut self, caps_mgmt_if: &[u8]) {
        let bridge_name = b"bridge-caps";
        let vxlan_if_name = b"vxlan-caps";
        //let caps_mgmt_if = b"caps-vlan99-mgmt";
        let caps_mgmt_vlan = 99;
        let bridge = self.bridge.entry(bridge_name.into()).or_default();
        let mgmt_vlan = self.vlans.entry(caps_mgmt_if.into()).or_default();
        mgmt_vlan.interface = bridge_name.into();
        mgmt_vlan.vlan_id = caps_mgmt_vlan;
        bridge.protocol_mode = InterfaceBridgeProtocolMode::Mstp;
        bridge.vlan_filtering = true;
        let bridge_vxlan_port = self
            .bridge_port
            .entry((bridge_name.into(), vxlan_if_name.into()))
            .or_default();
        bridge_vxlan_port.frame_types = VlanFrameTypes::AdmitOnlyVlanTagged;
        self.bridge_vlans.retain(|vlan| {
            vlan.bridge.0.as_ref() != bridge_name && !vlan.tagged.contains(&vxlan_if_name.into())
        });
        self.bridge_vlans.push(InterfaceBridgeVlanCfg {
            bridge: bridge_name.into(),
            tagged: [AsciiString::from(vxlan_if_name)].into(),
            vlan_ids: [PossibleRangeDash::from(1..4096)].into(),
            ..InterfaceBridgeVlanCfg::default()
        });
        self.wifi_cap.enabled = YesNo::Yes;
        self.wifi_cap.discovery_interfaces = Some(caps_mgmt_if.into()).into_iter().collect();
        // remove direct wifi settings
        self.wifi = Box::default();
    }
    fn bridges(&self) -> impl Iterator<Item = InterfaceBridgeByName> {
        self.bridge.iter().map(|(name, b)| {
            let mut bridge = b.clone();
            bridge.name = name.clone();
            InterfaceBridgeByName(bridge)
        })
    }
    fn vxlans(&self) -> impl Iterator<Item = InterfaceVxlanByName> {
        self.vxlan.iter().map(|(name, cfg)| {
            let mut vxlan = cfg.clone();
            vxlan.name = name.clone();
            InterfaceVxlanByName(vxlan)
        })
    }
    fn vxlan_vteps(&self) -> impl Iterator<Item = InterfaceVxlanVtepsCfg> {
        self.vxlan_vteps.iter().map(|((interface, addr), cfg)| {
            let mut vxlan = cfg.clone();
            vxlan.interface = interface.clone();
            vxlan.remote_ip = addr.clone();
            vxlan
        })
    }
    fn bridge_ports(&self) -> impl Iterator<Item = InterfaceBridgePortCfg> {
        self.bridge_port.iter().map(|((bridge_name, if_name), b)| {
            let mut port = b.clone();
            port.bridge = bridge_name.clone();
            port.interface = if_name.clone();
            port
        })
    }
    fn vlans(&self) -> impl Iterator<Item = InterfaceVlanByName> {
        self.vlans.iter().map(|(name, v)| {
            let mut vlan = v.clone();
            vlan.name = name.clone();
            InterfaceVlanByName(vlan)
        })
    }

    fn generate_mutations<'a>(
        &'a self,
        from: &'a DeviceDataCurrent,
    ) -> Result<Box<[ResourceMutation<'a>]>, ResourceMutationError<'a>> {
        let remaining_updates = generate_add_update_remove_by_key(
            &from.vxlan,
            self.vxlans().map(Cow::<InterfaceVxlanByName>::Owned),
        )
        .chain(generate_add_update_remove_by_id(
            &from.vxlan_vteps,
            self.vxlan_vteps().map(Cow::<InterfaceVxlanVtepsCfg>::Owned),
        ))
        .chain(generate_add_update_remove_by_key(
            &from.bridge,
            self.bridges().map(Cow::<InterfaceBridgeByName>::Owned),
        ))
        .chain(generate_add_update_remove_by_id(
            &from.bridge_port,
            self.bridge_ports()
                .map(Cow::<InterfaceBridgePortCfg>::Owned),
        ))
        .chain(Some(generate_single_update(&from.identity, &self.identity)))
        .chain(Some(generate_single_update(
            &from.routerboard_settings,
            &self.routerboard_settings,
        )))
        .chain(Some(generate_single_update(&from.wifi_cap, &self.wifi_cap)))
        .chain(generate_update_by_key(&from.wifi, &self.wifi)?)
        .chain(generate_add_update_remove_by_key(
            &from.wifi_datapath,
            self.wifi_datapath.iter().map(Cow::Borrowed),
        ))
        .chain(generate_update_by_key(&from.ethernet, &self.ethernet)?)
        .chain(generate_add_update_remove_by_key(
            &from.vlans,
            self.vlans().map(Cow::<InterfaceVlanByName>::Owned),
        ))
        .collect();
        Ok(remaining_updates)
    }
}

impl DeviceDataCurrent {
    async fn fetch(device: &MikrotikDevice) -> Result<Self, resource::Error> {
        Ok(Self {
            identity: SystemIdentityCfg::fetch(device)
                .await?
                .expect("System identity not found"),
            routerboard_settings: SystemRouterboardSettingsCfg::fetch(device)
                .await?
                .expect("system routerboard settings not found"),
            ethernet: InterfaceEthernetByDefaultName::fetch_all(device).await?,
            wifi: InterfaceWifiByDefaultName::fetch_all(device).await?,
            wifi_cap: InterfaceWifiCapCfg::fetch(device)
                .await?
                .expect("Wifi cap not found"),
            wifi_datapath: InterfaceWifiDatapathByName::fetch_all(device).await?,
            bridge: InterfaceBridgeByName::fetch_all(device).await?,
            bridge_port: InterfaceBridgePortById::fetch_all(device).await?,
            vlans: vec![],
            vxlan: InterfaceVxlanByName::fetch_all(device).await?,
            vxlan_vteps: InterfaceVxlanVtepsById::fetch_all(device).await?,
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .parse_env(Env::default().filter_or("LOG_LEVEL", "info"))
        .format_timestamp(Some(TimestampPrecision::Millis))
        .init();

    let wlan_hosts: [IpAddr; 6] = [
        ip_addr!(v4, "172.16.1.1").into(),
        ip_addr!(v4, "10.172.12.253").into(),
        ip_addr!(v4, "10.172.12.252").into(),
        ip_addr!(v4, "10.192.9.238").into(),
        ip_addr!(v4, "10.192.69.2").into(),
        ip_addr!(v4, "172.17.0.34").into(),
    ];

    let mut target_data = DeviceDataTarget::new(DeviceType::C52iG_5HaxD2HaxD);

    target_data.set_identity(b"ap-buero");
    target_data.enable_vxlan(
        b"vxlan-caps",
        1,
        wlan_hosts
            .iter()
            .filter(|a| *a != &IpAddr::V4(ip_addr!(v4, "10.192.69.2"))),
    );

    target_data.enable_wifi_cap(b"caps-vlan99-mgmt");

    /*target_data
        .interface_vxlan_by_name
        .push(InterfaceVxlanByName(InterfaceVxlanCfg {
            name: b"vxlan1".into(),
            comment: Some(b"Generated Config".into()),
            ..InterfaceVxlanCfg::default()
        }));
    target_data
        .interface_vxlan_vteps_cfg
        .push(InterfaceVxlanVtepsCfg {
            comment: Default::default(),
            interface: b"vxlan1".into(),
            port: 8472,
            remote_ip: IpAddr::V4(Ipv4Addr::new(172, 16, 1, 1)),
        });*/

    //target_data.interface_bridge_by_name.push(InterfaceBridgeByName(InterfaceBridgeCfg::new()))

    let cfg = Config::builder()
        .add_source(File::with_name("routers.yaml"))
        .add_source(
            Environment::with_prefix("APP")
                .separator("-")
                .prefix_separator("_"),
        )
        .build()?;
    let credentials: Credentials = cfg.get("credentials")?;
    let router = IpAddr::V4(Ipv4Addr::new(10, 192, 5, 7));
    //let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 51));
    //let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 54));
    //let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 1));
    let router = IpAddr::V4(Ipv4Addr::new(10, 192, 69, 2));
    println!("{router}");
    let device = MikrotikDevice::connect(
        (router, 8728),
        credentials.user.as_bytes(),
        Some(credentials.password.as_bytes()),
    )
    .await?;
    let current_data = DeviceDataCurrent::fetch(&device).await?;

    //let current_vxlan_by_name: Box<[_]> = InterfaceVxlanByName::fetch_all(&device).await?;

    let remaining_updates = match target_data.generate_mutations(&current_data) {
        Ok(mutations) => mutations,
        Err(error) => {
            panic!("Error:  {error}")
        }
    };

    match sort_mutations(remaining_updates.as_ref()) {
        Ok(mutations) => {
            let mut cfg = String::new();
            let mut generator = Generator::new(&mut cfg);
            for mutation in mutations {
                generator.append_mutation(mutation)?;
            }
            info!("Mutations: \n{cfg}");
        }
        Err(error) => {
            error!("Error:  {error}")
        }
    }

    Ok(())
}

fn sort_mutations<'a, 'b>(
    updates: &'b [ResourceMutation<'a>],
) -> Result<Box<[&'b ResourceMutation<'a>]>, MissingDependenciesError<'a, 'b>> {
    let mut remaining_updates = updates.iter().collect::<Vec<_>>();
    let mut sorted_mutations = Vec::with_capacity(updates.len());
    let mut provided_dependencies = HashSet::new();
    while !remaining_updates.is_empty() {
        let mut next_round = Vec::with_capacity(remaining_updates.len());
        let mut could_add = false;
        for mutation in remaining_updates {
            if mutation
                .depends
                .iter()
                .all(|dep| provided_dependencies.contains(dep))
            {
                for dep in &mutation.provides {
                    info!(
                        "{} provides {:?}:{}",
                        decode_latin1(mutation.resource),
                        dep.0,
                        decode_latin1(dep.1.as_ref())
                    );
                    provided_dependencies.insert(dep.clone());
                }
                sorted_mutations.push(mutation);
                could_add = true;
            } else {
                next_round.push(mutation);
            }
        }
        if !could_add {
            let dependencies = next_round
                .iter()
                .flat_map(|m| {
                    m.depends
                        .iter()
                        .filter(|dep| !provided_dependencies.contains(dep))
                })
                .cloned()
                .collect();
            return Err(MissingDependenciesError {
                dependencies,
                unresolved_mutations: next_round.into_boxed_slice(),
            });
        }
        remaining_updates = next_round;
    }
    Ok(sorted_mutations.into_boxed_slice())
}
