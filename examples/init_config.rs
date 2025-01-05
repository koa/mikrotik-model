use config::{Config, Environment, File};
use env_logger::{Env, TimestampPrecision};
use mikrotik_model::generator::Generator;
use mikrotik_model::hwconfig::DeviceType;
use mikrotik_model::model::Data;
use mikrotik_model::resource::{KeyedResource, Updatable};
use mikrotik_model::{Credentials, MikrotikDevice};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};

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
    let device = MikrotikDevice::connect(
        (router, 8728),
        credentials.user.as_bytes(),
        Some(credentials.password.as_bytes()),
    )
    .await?;
    let current_data = Data::fetch_from_device(&device).await?;
    let existing_data = &current_data.interface_ethernet_by_default_name;
    println!("Current data: {:#?}", existing_data);

    let data = DeviceType::RB750Gr3.generate_empty_data();
    let new_data = &data.interface_ethernet_by_default_name;

    let mut new_entries: HashMap<_, _> = new_data
        .iter()
        .map(|e| (e.key_value().clone(), e.clone()))
        .collect();
    let mut cfg = String::new();
    let mut generator = Generator::new(&mut cfg);

    for existing_entry in existing_data.iter() {
        if let Some(new_entry) = new_entries.remove(existing_entry.key_value()) {
            generator.append_mutation(&new_entry.calculate_update(existing_entry))?;
        }
    }
    println!("{cfg}");
    Ok(())
}
