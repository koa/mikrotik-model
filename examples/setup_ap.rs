use config::{Config, Environment, File};
use env_logger::{Env, TimestampPrecision};
use log::{error, info};
use mikrotik_model::{
    generator::Generator,
    hwconfig::DeviceType,
    model::{Data, InterfaceVxlanByName, InterfaceVxlanCfg, InterfaceVxlanVtepsCfg, YesNo},
    resource,
    resource::{KeyedResource, UpdatePairing},
    Credentials, MikrotikDevice,
};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .parse_env(Env::default().filter_or("LOG_LEVEL", "info"))
        .format_timestamp(Some(TimestampPrecision::Millis))
        .init();

    let mut target_data = DeviceType::C52iG_5HaxD2HaxD.generate_empty_data();

    target_data
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
        });

    //target_data.interface_bridge_by_name.push(InterfaceBridgeByName(InterfaceBridgeCfg::new()))
    target_data.interface_wifi_cap_cfg.enabled = YesNo::Yes;

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
    //let router = IpAddr::V4(Ipv4Addr::new(10, 192, 69, 2));
    println!("{router}");
    let device = MikrotikDevice::connect(
        (router, 8728),
        credentials.user.as_bytes(),
        Some(credentials.password.as_bytes()),
    )
    .await?;
    //let mut current_data = Data::fetch_from_device(&device).await?;

    let mut current_vxlan_by_name: Box<[_]> = InterfaceVxlanByName::fetch_all(&device).await?;

    let mut cfg = String::new();
    let mut generator = Generator::new(&mut cfg);

    let vxlan_updates = UpdatePairing::match_updates_by_key(
        &mut current_vxlan_by_name,
        &target_data.interface_vxlan_by_name,
    );

    info!("vxlan_updates: {vxlan_updates:#?}");

    let mut remaining_updates = vxlan_updates.generate_updates().collect::<Vec<_>>();
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
                generator.append_mutation(&mutation)?;
                for dep in mutation.provides {
                    provided_dependencies.insert(dep);
                }
                could_add = true;
            } else {
                next_round.push(mutation);
            }
        }
        if !could_add {
            error!("Could not resolve dependencies");
            break;
        }
        remaining_updates = next_round;
    }

    info!("Mutations: \n{cfg}");

    Ok(())
}
