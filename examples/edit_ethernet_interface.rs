use config::{Config, Environment, File};
use env_logger::{Env, TimestampPrecision};
use mikrotik_model::{
    Credentials, MikrotikDevice,
    generator::Generator,
    hwconfig::DeviceType,
    model::Data,
    resource::{KeyedResource, UpdatePairing},
};
use std::{
    borrow::Cow,
    net::{IpAddr, Ipv4Addr},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .parse_env(Env::default().filter_or("LOG_LEVEL", "info"))
        .format_timestamp(Some(TimestampPrecision::Millis))
        .init();

    let target_data = DeviceType::C52iG5haxD2haxD.generate_empty_data();

    let cfg = Config::builder()
        .add_source(File::with_name("routers.yaml"))
        .add_source(
            Environment::with_prefix("APP")
                .separator("-")
                .prefix_separator("_"),
        )
        .build()?;
    let credentials: Credentials = cfg.get("credentials")?;
    //let router = IpAddr::V4(Ipv4Addr::new(10, 192, 5, 7));
    //let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 51));
    //let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 54));
    let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 1));
    //let router = IpAddr::V4(Ipv4Addr::new(10, 192, 69, 2));
    println!("{router}");
    let device = MikrotikDevice::connect(
        (router, 8728),
        credentials.user.as_bytes(),
        Some(credentials.password.as_bytes()),
    )
    .await?;

    let mut cfg = String::new();
    let mut generator = Generator::new(&mut cfg);

    let current_data = Data::fetch_from_device(&device).await?;

    let ethernet_updates = UpdatePairing::match_updates_by_key(
        &current_data.interface_ethernet_by_default_name,
        target_data
            .interface_ethernet_by_default_name
            .iter()
            .map(Cow::Borrowed),
    );
    dump_changes(&ethernet_updates);

    for result in ethernet_updates.generate_updates_or_error() {
        match result {
            Ok(mutation) => generator.append_mutation(&mutation)?,
            Err(e) => {
                println!("Ethernet update Error: {}", e);
            }
        }
    }

    let wifi_updates = UpdatePairing::match_updates_by_key(
        &current_data.interface_wifi_by_default_name,
        target_data
            .interface_wifi_by_default_name
            .iter()
            .map(Cow::Borrowed),
    );
    dump_changes(&wifi_updates);
    for result in wifi_updates.generate_updates_or_error() {
        match result {
            Ok(mutation) => generator.append_mutation(&mutation)?,
            Err(e) => {
                println!("Wifi update Error: {}", e);
            }
        }
    }

    /*let res: Box<[_]> =
        <(InterfaceEthernetByDefaultName, InterfaceEthernetState)>::fetch_all(&device).await?;
    let mut cfg = String::new();
    let mut generator = Generator::new(&mut cfg);
    let mut ether_idx = 0;
    let mut sfp_idx = 0;
    let mut qsfp_idx = 0;
    for (interface, status) in res.iter() {
        let mut new_if = interface.clone();
        let default_name = status.default_name.0.as_ref();
        if default_name.starts_with(b"sfp") {
            sfp_idx += 1;
            new_if.data.name = AsciiString(Box::from(format!("s{:02}", sfp_idx).as_bytes()));
        } else if default_name.starts_with(b"ether") {
            ether_idx += 1;
            new_if.data.name = format!("e{:02}", ether_idx).as_bytes().into();
        } else if default_name.starts_with(b"qsfp") {
            qsfp_idx += 1;
            new_if.data.name = format!("q{:02}", qsfp_idx).as_bytes().into();
        } else {
            println!("Default: {}", decode_latin1(default_name));
        }

        //new_if.0.name = format!("e{:02}", idx + 1).into();
        new_if.data.advertise = [
            EthernetSpeed::_100MBaseTFull,
            EthernetSpeed::_100MBaseTHalf,
            EthernetSpeed::_1GBaseTFull,
        ]
        .into();
        generator.append_mutation(&new_if.calculate_update(interface))?;
    }*/
    println!("{cfg}");
    Ok(())
}

fn dump_changes<T: KeyedResource + mikrotik_model::resource::CfgResource + std::clone::Clone>(
    updated: &UpdatePairing<T, T>,
) where
    <T as KeyedResource>::Key: std::fmt::Debug,
{
    if !updated.orphaned_entries.is_empty() {
        println!("orphan entries: {}", updated.orphaned_entries.len());
        for eth in updated.orphaned_entries.iter() {
            println!(" - {:?}", eth.key_value());
        }
    }
    if !updated.new_entries.is_empty() {
        println!("new entries: {}", updated.new_entries.len());
    }
}
