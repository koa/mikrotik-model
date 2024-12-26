use config::{Config, Environment, File};
use env_logger::{Env, TimestampPrecision};
use itertools::Itertools;
use mikrotik_model::generator::Generator;
use mikrotik_model::model::{InterfaceEthernetByName, InterfaceEthernetState};
use mikrotik_model::resource::{KeyedResource, Updatable};
use mikrotik_model::Credentials;
use mikrotik_rs::MikrotikDevice;
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
use tokio_stream::StreamExt;
use mikrotik_model::model::InterfaceEthernetById;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .parse_env(Env::default().filter_or("LOG_LEVEL", "info"))
        .format_timestamp(Some(TimestampPrecision::Millis))
        .init();

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
    println!("{router}");
    let device = MikrotikDevice::connect(
        (router, 8728),
        credentials.user.as_ref(),
        Some(credentials.password.as_ref()),
    )
    .await?;

    let res: Box<[_]> =
        <(InterfaceEthernetById, InterfaceEthernetState)>::fetch_all(&device).await?;
    let mut cfg = String::new();
    let mut generator = Generator::new(&mut cfg);
    let mut ether_idx = 0;
    let mut sfp_idx = 0;
    let mut qsfp_idx = 0;
    for (interface, status) in res.iter() {
        let mut new_if = interface.clone();
        let default_name = status.default_name.as_ref();
        if default_name.starts_with("sfp") {
            sfp_idx += 1;
            new_if.data.name = format!("s{:02}", sfp_idx).into();
        } else if default_name.starts_with("ether") {
            ether_idx += 1;
            new_if.data.name = format!("e{:02}", ether_idx).into();
        } else if default_name.starts_with("qsfp") {
            qsfp_idx += 1;
            new_if.data.name = format!("q{:02}", qsfp_idx).into();
        } else {
            println!("Default: {}", default_name);
        }

        //new_if.0.name = format!("e{:02}", idx + 1).into();
        //new_if.0.advertise=HashSet::from([EthernetSpeed::_100MBaseTFull, EthernetSpeed::_100MBaseTHalf, EthernetSpeed::_1GBaseTFull]);
        generator.append_mutation(&new_if.calculate_update(interface))?;
    }
    println!("{cfg}");
    Ok(())
}
