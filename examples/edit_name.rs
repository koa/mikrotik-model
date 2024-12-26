use config::{Config, Environment, File};
use env_logger::{Env, TimestampPrecision};
use itertools::Itertools;
use mikrotik_model::model::SystemIdentityCfg;
use mikrotik_model::resource::DeserializeRosResource;
use mikrotik_model::resource::{list_resources, CfgResource};
use mikrotik_model::value::{write_script_string, KeyValuePair};
use mikrotik_model::Credentials;
use mikrotik_rs::protocol::command::{Cmd, CommandBuilder};
use mikrotik_rs::MikrotikDevice;
use std::net::{IpAddr, Ipv4Addr};
use tokio_stream::wrappers::ReceiverStream;
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
    let router = IpAddr::V4(Ipv4Addr::new(10, 192, 5, 7));
    println!("{router}");
    let device = MikrotikDevice::connect(
        (router, 8728),
        credentials.user.as_ref(),
        Some(credentials.password.as_ref()),
    )
    .await?;
    if let Some(entry) = list_resources::<SystemIdentityCfg>(&device)
        .await
        .next()
        .await
    {
        println!("{:#?}", entry);
        let new_entry = SystemIdentityCfg {
            name: "Guetä Morgä".into(),
        };
        println!("{:#?}", new_entry);

        let cmd = CommandBuilder::new()
            .command(&format!("/{}/set", SystemIdentityCfg::path()))?
            .build();
        let mut cmd: Option<CommandBuilder<Cmd>> = None;
        let mut update_str: Option<String> = None;

        for KeyValuePair { key, value } in new_entry.changed_values(&entry) {
            let update = if let Some(value) = update_str.as_mut() {
                value.push(' ');
                value
            } else {
                update_str = Some(format!("/{}\nset ", SystemIdentityCfg::path()));
                update_str.as_mut().expect("Something went wrong")
            };
            update.push_str(key);
            update.push('=');
            write_script_string(update, value.as_ref())?;
            cmd = Some(
                cmd.take()
                    .unwrap_or_else(|| {
                        CommandBuilder::new()
                            .command(&format!("/{}/set", SystemIdentityCfg::path()))
                            .unwrap()
                    })
                    .attribute(key, Some(value.as_ref()))?,
            );
        }
        if let Some(update) = update_str {
            println!("{update}");
        }
        if let Some(cmd) = cmd {
            let result: Box<[_]> = ReceiverStream::new(device.send_command(cmd.build()).await)
                .collect()
                .await;
            println!("Update result: {:#?}", result);
        }
    }
    Ok(())
}
