use config::{Config, Environment, File};
use env_logger::{Env, TimestampPrecision};
use log::{error, info};
use mikrotik_model::{
    ascii::AsciiString, generator::Generator, hwconfig::DeviceType, resource::ResourceMutation, Credentials,
    MikrotikDevice,
};
use mikrotik_model_generator_macro::mikrotik_model;
use std::net::{IpAddr, Ipv4Addr};

mikrotik_model!(
    name = DeviceData,
    fields(
        identity(single = "system/identity"),
        bridge(by_key(path = "interface/bridge", key = name)),
        bridge_port(by_id(
            path = "interface/bridge/port",
            keys(bridge, interface)
        ))
    ),
);

impl DeviceDataTarget {
    fn new(device_type: DeviceType) -> Self {
        Self {
            identity: Default::default(),
            bridge_port: Default::default(),
        }
    }
    fn set_identity(&mut self, name: impl Into<AsciiString>) {
        self.identity.name = name.into();
    }
}

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
    info!("Current device: {:#?}", current_data);
    let mut target_data = DeviceDataTarget::new(DeviceType::C52iG_5HaxD2HaxD);

    target_data.set_identity(b"ap-buero");
    let remaining_updates = match target_data.generate_mutations(&current_data) {
        Ok(mutations) => mutations,
        Err(error) => {
            panic!("Error:  {error}")
        }
    };

    match ResourceMutation::sort_mutations(remaining_updates.as_ref()) {
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
