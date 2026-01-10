use config::{Config, Environment, File};
use encoding_rs::mem::decode_latin1;
use env_logger::{Env, TimestampPrecision};
use log::{error, info, warn};
use mikrotik_model::ascii::AsciiString;
use mikrotik_model::model::{
    CapsManInterfaceById, CapsManInterfaceCfg, CapsManInterfaceState, CapsManRadioState,
    CapsManRegistrationTableState, Data,
};
use mikrotik_model::resource::{SentenceResult, stream_resource};
use mikrotik_model::{Credentials, MikrotikDevice};
use std::borrow::Cow;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use tokio_stream::StreamExt;

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
    //let router = IpAddr::V4(Ipv4Addr::new(10, 192, 5, 7));
    //let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 51));
    let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 1));
    let device: MikrotikDevice = MikrotikDevice::connect(
        (router, 8728),
        credentials.user.as_bytes(),
        Some(credentials.password.as_bytes()),
    )
    .await?;

    info!("============== Capsman Radio ==============");
    let mut cap_identities_by_radio = HashMap::new();
    let mut radio_stream = stream_resource::<CapsManRadioState>(&device)
        .await
        .filter_map(log_problems);
    while let Some(radio) = radio_stream.next().await {
        cap_identities_by_radio.insert(radio.interface.clone(), radio);
    }
    let mut configuration_by_interface = HashMap::new();
    info!("============== Capsman Interfaces ==============");
    let mut interface_stream =
        stream_resource::<(CapsManInterfaceById, CapsManInterfaceState)>(&device)
            .await
            .filter_map(log_problems);
    while let Some((cfg, state)) = interface_stream.next().await {
        //info!("Found Interface: {:?}, {:?}", cfg,state);
        let cfg = cfg.data;
        configuration_by_interface.insert(cfg.name.clone(), (cfg, state));
    }

    info!("============== Capsman Registrations ==============");

    let mut registration_table = stream_resource::<CapsManRegistrationTableState>(&device)
        .await
        .filter_map(log_problems);
    while let Some(value) = registration_table.next().await {
        let if_name = &value.interface;
        let if_data = configuration_by_interface.get(if_name);
        let master_interface = if_data
            .and_then(|(i, _)| i.master_interface.as_ref())
            .and_then(|n| n.value())
            .unwrap_or(if_name);
        let cap_identity_option = cap_identities_by_radio
            .get(master_interface)
            .map(|i| &i.remote_cap_identity)
            .map(<&AsciiString>::into);
        let cap_identity: &str = cap_identity_option
            .as_ref()
            .map(Cow::as_ref)
            .unwrap_or_default();
        let cfg_name_option= if_data
            .and_then(|(i, _)| i.configuration.as_ref())
            .map(<&AsciiString>::into);
        let cfg_name = cfg_name_option
            .as_ref()
            .map(Cow::as_ref)
            .unwrap_or_default();
        println!("{}, {cap_identity}, {}, {if_name}", value.mac_address, value.ssid);
    }

    Ok(())
}

fn log_problems<E>(entry: SentenceResult<E>) -> Option<E> {
    match entry {
        SentenceResult::Row { value, warnings } => {
            warnings.iter().for_each(|w| warn!("Warning: {w}"));
            Some(value)
        }

        SentenceResult::Error { errors, warnings } => {
            warnings.iter().for_each(|w| warn!("Warning: {w}"));
            for error in errors.iter() {
                error!("Error: {error}");
            }
            None
        }
        SentenceResult::Trap { category, message } => {
            error!("Trap: {category:?}: {}", decode_latin1(message.as_ref()));
            None
        }
    }
}
