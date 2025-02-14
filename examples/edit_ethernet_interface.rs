use config::{Config, Environment, File};
use env_logger::{Env, TimestampPrecision};
use mikrotik_model::hwconfig::DeviceType;
use mikrotik_model::model::InterfaceWifiByDefaultName;
use mikrotik_model::resource::{DeserializeRosBuilder, Updatable};
use mikrotik_model::{
    ascii,
    ascii::AsciiString,
    generator::Generator,
    model::InterfaceEthernetByDefaultName,
    model::ReferenceType,
    resource::{FieldUpdateHandler, KeyedResource, SetResource},
    value::{KeyValuePair, RosValue},
    Credentials, MikrotikDevice,
};
use std::borrow::Cow;
use std::fmt::Write;
use std::iter::repeat;
use std::net::{IpAddr, Ipv4Addr};

struct InterfaceEthernetSet {
    pub name: AsciiString,
    pub default_name: AsciiString,
}

impl FieldUpdateHandler for InterfaceEthernetSet {
    fn update_reference<V: RosValue + 'static>(
        &mut self,
        ref_type: ReferenceType,
        old_value: &V,
        new_value: &V,
    ) -> bool {
        let old_value_any = old_value as &dyn core::any::Any;
        let new_value_any = new_value as &dyn core::any::Any;

        if let ReferenceType::Interface = ref_type {
            if let (Some(old_value), Some(new_value)) = (
                old_value_any.downcast_ref::<ascii::AsciiString>(),
                new_value_any.downcast_ref::<ascii::AsciiString>(),
            ) {
                let mut modified = false;
                if old_value == &self.name {
                    self.name = new_value.clone();
                    modified = true;
                }
                modified
            } else {
                false
            }
        } else {
            false
        }
    }
}

impl SetResource<InterfaceEthernetByDefaultName> for InterfaceEthernetSet {
    fn changed_values(
        &self,
        before: &InterfaceEthernetByDefaultName,
    ) -> impl Iterator<Item = KeyValuePair> {
        let mut ret = Vec::new();
        if before.data.name != self.name {
            ret.push(KeyValuePair {
                key: b"name",
                value: Cow::Borrowed(&self.name.0),
            })
        }
        ret.into_iter()
    }
}

struct UpdatePairing<'a, 'b, Cfg: KeyedResource, Set: KeyedResource> {
    orphaned_entries: Box<[&'a Cfg]>,
    matched_entries: Box<[(&'a Cfg, &'b Set)]>,
    new_entries: Box<[&'b Set]>,
}

impl<'b, Cfg: KeyedResource, Set: KeyedResource + Updatable<From = Cfg>>
    UpdatePairing<'b, 'b, Cfg, Set>
{
    fn generate_updates<W: Write>(&self, generator: &mut Generator<W>) -> std::fmt::Result {
        for (original, target) in &self.matched_entries {
            let mutation = target.calculate_update(original);
            generator.append_mutation(&mutation)?;
        }
        Ok(())
    }
}

fn match_updates<'a, 'b, T: KeyedResource>(
    original: &'a [T],
    target: &'b [T],
) -> UpdatePairing<'a, 'b, T, T>
where
    T::Key: PartialEq,
{
    let mut orphans = Vec::with_capacity(original.len());
    let mut matched = Vec::with_capacity(original.len().max(target.len()));
    let mut new = Vec::with_capacity(target.len());
    let mut available_targets: Box<[bool]> = repeat(true).take(target.len()).collect();
    'original: for o in original {
        let key = o.key_value();
        for (idx, t) in target
            .iter()
            .enumerate()
            .filter(|(idx, _)| available_targets[*idx])
        {
            if t.key_value() == key {
                matched.push((o, t));
                available_targets[idx] = false;
                continue 'original;
            }
        }
        orphans.push(o);
    }
    target
        .iter()
        .enumerate()
        .filter(|(idx, _)| available_targets[*idx])
        .for_each(|(_, v)| new.push(v));
    UpdatePairing {
        orphaned_entries: orphans.into_boxed_slice(),
        matched_entries: matched.into_boxed_slice(),
        new_entries: new.into_boxed_slice(),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .parse_env(Env::default().filter_or("LOG_LEVEL", "info"))
        .format_timestamp(Some(TimestampPrecision::Millis))
        .init();

    let target_interfaces = [
        InterfaceEthernetSet {
            default_name: b"ether1".into(),
            name: b"e01".into(),
        },
        InterfaceEthernetSet {
            default_name: b"ether2".into(),
            name: b"e02".into(),
        },
        InterfaceEthernetSet {
            default_name: b"ether3".into(),
            name: b"e03".into(),
        },
        InterfaceEthernetSet {
            default_name: b"ether4".into(),
            name: b"e04".into(),
        },
        InterfaceEthernetSet {
            default_name: b"ether5".into(),
            name: b"e05".into(),
        },
    ];

    let data = DeviceType::C52iG_5HaxD2HaxD.generate_empty_data();

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
    //let router = IpAddr::V4(Ipv4Addr::new(172, 16, 1, 1));
    let router = IpAddr::V4(Ipv4Addr::new(10, 192, 69, 2));
    println!("{router}");
    let device = MikrotikDevice::connect(
        (router, 8728),
        credentials.user.as_bytes(),
        Some(credentials.password.as_bytes()),
    )
    .await?;

    let mut cfg = String::new();
    let mut generator = Generator::new(&mut cfg);

    let current_ethernet_interfaces: Box<[_]> =
        <InterfaceEthernetByDefaultName>::fetch_all(&device).await?;
    let ethernet_updates = match_updates(
        &current_ethernet_interfaces,
        &data.interface_ethernet_by_default_name,
    );
    dump_changes(&ethernet_updates);

    ethernet_updates.generate_updates(&mut generator)?;

    let current_wifi_interfaces: Box<[_]> =
        <InterfaceWifiByDefaultName>::fetch_all(&device).await?;
    let wifi_updates = match_updates(
        &current_wifi_interfaces,
        &data.interface_wifi_by_default_name,
    );
    dump_changes(&wifi_updates);
    wifi_updates.generate_updates(&mut generator)?;

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

fn dump_changes<T: KeyedResource>(updated: &UpdatePairing<T, T>)
where
    <T as KeyedResource>::Key: std::fmt::Debug,
{
    if !updated.orphaned_entries.is_empty() {
        println!(
            "orphan entries: {}",
            updated.orphaned_entries.len()
        );
        for eth in updated.orphaned_entries.iter() {
            println!(" - {:?}", eth.key_value());
        }
    }
    if !updated.new_entries.is_empty() {
        println!("new entries: {}", updated.new_entries.len());
    }
}
