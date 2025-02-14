use clap::Parser;
use convert_case::{Case, Casing};
use encoding_rs::mem::encode_latin1_lossy;
use env_logger::{Env, TimestampPrecision};
use lazy_static::lazy_static;
use log::error;
use mikrotik_api::prelude::MikrotikDevice;
use mikrotik_api::simple::SimpleResult;
use mikrotik_model_generator::known_entities;
use mikrotik_model_generator::model::{Entity, Field, Reference};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::net::IpAddr;
use std::path::Path;
use tokio_stream::StreamExt;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// device to contact
    device: IpAddr,

    /// login password
    #[arg(short, long)]
    password: Option<Box<str>>,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .parse_env(Env::default().filter_or("LOG_LEVEL", "info"))
        .format_timestamp(Some(TimestampPrecision::Millis))
        .init();
    let args = Args::parse();
    let device: MikrotikDevice<SimpleResult> = MikrotikDevice::connect(
        (args.device, 8728),
        b"admin",
        args.password.as_deref().map(|v| encode_latin1_lossy(v)),
    )
    .await?;

    let original_entities = known_entities().collect::<Vec<_>>();

    let mut remaining_entities = original_entities
        .iter()
        .map(|e| (e.path.clone(), e))
        .collect::<HashMap<_, _>>();
    let mut new_entities = Vec::new();
    walk_dir(&device, &[], &mut remaining_entities, &mut new_entities).await;
    //println!("{:?}", remaining_entities.keys());
    //walk_dir(&device, &["system".into()], &mut remaining_entities, &mut new_entities).await;
    /*guess_field(&device, &["interface", "ethernet"], "combo-mode").await;
    guess_field(&device, &["interface", "ethernet"], "advertise").await;
    guess_field(&device, &["interface", "ethernet"], "mtu").await;
    guess_field(&device, &["routing", "bgp", "connection"], "remote.as").await;
    guess_field(&device, &["interface", "bridge"], "priority").await;
    guess_field(&device, &["ip", "address"], "interface").await;
    guess_field(&device, &["interface", "bridge"], "priority").await;
    guess_field(&device, &["system", "routerboard","settings"], "preboot-etherboot").await;
    guess_field(&device, &["interface", "ethernet"], "bandwidth").await;*/
    let mut enums = HashMap::new();
    for entity in new_entities.iter() {
        for field in entity.fields.iter() {
            if let Some(inline_enum) = field.inline_enum.as_ref() {
                let mut variants = inline_enum.clone();
                variants.sort();
                enums
                    .entry(variants)
                    .or_insert((inline_enum.clone(), Vec::new()))
                    .1
                    .push((entity.path.clone(), field.name.clone()));
            }
        }
    }
    for (_, (values, users)) in enums {
        if users.len() > 1 {
            println!("------------------");
            println!("{}", values.join(", "));
            for (entity, field) in users {
                println!(" - {}: {field}", entity.join("/"));
            }
        }
    }

    let mut entries_by_group = HashMap::new();
    new_entities.iter().for_each(|e| {
        let path = &e.path;
        let group_path = if path.len() <= 2 {
            Box::new(path.as_ref())
        } else {
            Box::new(path[0..2].as_ref())
        };
        let name = if let [primary, secondary] = &group_path[..] {
            format!(
                "{}/{}",
                primary.as_ref().to_case(Case::Kebab),
                secondary.as_ref().to_case(Case::UpperCamel)
            )
        } else if let [primary] = &group_path[..] {
            format!(
                "{}/{}",
                primary.as_ref().to_case(Case::Kebab),
                primary.as_ref().to_case(Case::UpperCamel)
            )
        } else {
            "empty".to_owned()
        };
        entries_by_group.entry(name).or_insert(Vec::new()).push(e);
    });
    let base_dir = Path::new("target/ros_model");
    if !base_dir.exists() {
        create_dir_all(base_dir)?;
    }
    
    let mut entity_list_file = File::create( "target/ros_model/entities.txt")?;
    for (filename, entries) in entries_by_group {
        write!(&mut entity_list_file, "{filename}.txt\n")?;
        let path = format!("target/ros_model/{filename}.txt");
        let path = Path::new(&path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                create_dir_all(parent)?;
            }
        }
        let mut file = File::create(&path)?;
        for entity in entries {
            let mut content = String::new();
            entity.write_entity_lines(&mut content)?;
            file.write_all(content.as_bytes())?;
        }
    }

    Ok(())
}

async fn walk_dir<'a>(
    device: &MikrotikDevice<SimpleResult>,
    path: &[Box<str>],
    existing_entities: &mut HashMap<Box<[Box<str>]>, &Entity>,
    gathered_entities: &mut Vec<Entity>,
) {
    let mut stream = device
        .send_command(
            b"/console/inspect",
            |cmd| {
                let path = path.join(",");
                cmd.attribute(b"request", b"child")
                    .attribute(b"path", path.as_bytes())
            },
            (),
        )
        .await
        .filter_map(sentence_of);
    let mut children = Vec::new();
    let mut has_print = false;
    let mut can_add = false;
    let mut can_edit = false;
    let mut can_remove = false;
    let mut can_find = false;
    while let Some(mut sentence) = stream.next().await {
        let ty = sentence.remove("type").flatten();
        let node_type = sentence.remove("node-type").flatten();
        let name = sentence.remove("name").flatten();
        if let Some(name) = name {
            if Some("child") == ty.as_deref() {
                match node_type.as_deref() {
                    Some("cmd") => match name.as_ref() {
                        "print" => has_print = true,
                        "add" => can_add = true,
                        "set" => can_edit = true,
                        "find" => can_find = true,
                        "remove" => can_remove = true,
                        &_ => {}
                    },
                    Some("dir") => {
                        children.push(name);
                    }
                    Some("path") => {
                        children.push(name);
                    }
                    None => {}
                    Some(&_) => {}
                }
            }
        }
    }
    if has_print {
        gathered_entities.push(
            process_entry(
                device,
                path,
                can_add,
                can_edit,
                can_remove,
                can_find,
                existing_entities,
            )
            .await,
        );
    }

    for name in children {
        let entry_path = path
            .iter()
            .cloned()
            .chain(Some(name.clone()))
            .collect::<Box<[_]>>();
        Box::pin(walk_dir(
            device,
            &entry_path,
            existing_entities,
            gathered_entities,
        ))
        .await;
    }
}

async fn process_entry<'a>(
    device: &MikrotikDevice<SimpleResult>,
    path: &[Box<str>],
    can_add: bool,
    can_edit: bool,
    can_remove: bool,
    can_find: bool,
    existing_entities: &mut HashMap<Box<[Box<str>]>, &Entity>,
) -> Entity {
    println!("Processing {}", path.join("/"));
    let mut entity = existing_entities
        .remove(path)
        .cloned()
        .unwrap_or_else(|| Entity {
            path: Box::from(path),
            key_field: None,
            fields: vec![],
            is_single: false,
            can_add,
            no_default: false,
        });
    if !can_remove && !can_find && !can_add {
        entity.is_single = true;
    }
    let mut existing_fields = entity
        .fields
        .iter()
        .map(|f| (f.name.as_ref(), f))
        .collect::<HashMap<_, _>>();
    let mut stream = device
        .send_command(
            b"/console/inspect",
            |cmd| {
                let mut path = path.join(",");
                path.push_str(",get,value-name");
                cmd.attribute(b"request", b"completion")
                    .attribute(b"path", path.as_bytes())
            },
            (),
        )
        .await
        .filter_map(sentence_of);
    let mut ro_fields = HashSet::new();
    while let Some(sentence) = stream.next().await {
        let show = sentence
            .get("show")
            .into_iter()
            .flatten()
            .map(|s| s.as_ref() == "true")
            .next()
            .unwrap_or(false);
        if show {
            if let Some(name) = sentence.get("completion").into_iter().flatten().next() {
                ro_fields.insert(name.clone());
            }
        }
    }
    let mut stream = device
        .send_command(
            b"/console/inspect",
            |cmd| {
                let mut path = path.join(",");
                path.push_str(",set");
                cmd.attribute(b"request", b"child")
                    .attribute(b"path", path.as_bytes())
            },
            (),
        )
        .await
        .filter_map(sentence_of);
    let mut rw_fields = Vec::new();
    while let Some(mut sentence) = stream.next().await {
        let is_child = sentence.remove("type").flatten().as_deref() == Some("child");
        let is_arg = sentence.remove("node-type").flatten().as_deref() == Some("arg");
        let name = sentence
            .remove("name")
            .flatten()
            .filter(|n| n.as_ref() != "numbers");
        if is_child && is_arg {
            if let Some(name) = name {
                ro_fields.remove(&name);
                rw_fields.push(name);
            }
        }
    }
    let mut fields = Vec::new();
    for field_name in rw_fields {
        fields.push(guess_field(device, path, &field_name, &mut existing_fields).await);
        ro_fields.remove(&field_name);
    }
    existing_fields.values().cloned().cloned().for_each(|f| {
        ro_fields.remove(f.name.as_ref());
        fields.push(f);
    });
    for field_name in ro_fields {
        fields.push(Field {
            name: field_name,
            field_type: None,
            inline_enum: None,
            is_key: false,
            has_auto: false,
            is_set: false,
            is_range_dot: false,
            is_range_dash: false,
            is_optional: false,
            is_read_only: true,
            is_multiple: false,
            is_hex: false,
            reference: Default::default(),
            has_none: false,
            has_unlimited: false,
            has_disabled: false,
            is_rxtx_pair: false,
            keep_if_none: false,
            default: None,
        });
    }
    entity.fields = fields;

    entity
}

lazy_static! {
    static ref NUMBER_RANGE_REGEX: Regex =
        Regex::new(r"^(-?[0-9])+\.\.(-?[0-9]+).*").expect("Error in regex");
    static ref HEX_NUMBER_RANGE_REGEX: Regex =
        Regex::new(r"^(-?[0-9A-F])+\.\.(-?[0-9A-F]+).*").expect("Error in regex");
}

async fn guess_field(
    device: &MikrotikDevice<SimpleResult>,
    path: &[Box<str>],
    field_name: &str,
    existing_fields: &mut HashMap<&str, &Field>,
) -> Field {
    let mut stream = device
        .send_command(
            b"/console/inspect",
            |cmd| {
                let mut path = path.join(",");
                path.push_str(",set,");
                path.push_str(field_name);
                cmd.attribute(b"request", b"syntax,completion")
                    .attribute(b"path", path.as_bytes())
            },
            (),
        )
        .await
        .filter_map(sentence_of);
    let mut example_values = Vec::new();
    let mut number_type = None;
    let mut symbols = HashSet::new();
    let mut any_value_allowed = false;
    let mut has_star = false;
    while let Some(mut sentence) = stream.next().await {
        match sentence.remove("type").flatten().as_deref() {
            Some("completion") => {
                if sentence.remove("show").flatten().as_deref() == Some("true") {
                    if let Some(value) = sentence.remove("completion").flatten() {
                        let style = sentence.remove("style").flatten();
                        if let Some("") | Some("arg") | Some("none") = style.as_deref() {
                            example_values.push(value);
                        }
                    }
                } else if let Some(value) = sentence.remove("completion").flatten() {
                    match value.as_ref() {
                        "0x" => {
                            if number_type.is_none() {
                                number_type = Some(("i64", true));
                            } else {
                                number_type = number_type.map(|(t, _)| (t, true));
                            }
                        }
                        "<number>" => {
                            if number_type.is_none() {
                                number_type = Some(("i64", false));
                            }
                        }
                        "<value>" => any_value_allowed = true,
                        "*" => has_star = true,
                        &_ => {}
                    }
                }
            }
            Some("syntax") => {
                if let Some(symbol) = sentence
                    .remove("symbol")
                    .flatten()
                    .filter(|s| !s.is_empty())
                {
                    symbols.insert(symbol);
                }
                let text = sentence.remove("text").flatten();
                if let Some(text) = text.as_deref() {
                    let number_range = if let Some(found) = NUMBER_RANGE_REGEX.captures(text) {
                        let (_, [min, max]) = found.extract();
                        let min_value: i128 = min.parse::<i128>().expect("error in number");
                        let max_value: i128 = max.parse::<i128>().expect("error in number");
                        Some((min_value, max_value, false))
                    } else if let Some(found) = HEX_NUMBER_RANGE_REGEX.captures(text) {
                        let (_, [min, max]) = found.extract();
                        let min_value = i128::from_str_radix(min, 16).expect("error in number");
                        let max_value = i128::from_str_radix(max, 16).expect("error in number");
                        Some((min_value, max_value, true))
                    } else {
                        None
                    };
                    if let Some((min_value, max_value, hex)) = number_range {
                        number_type =
                            if in_range(min_value, max_value, u8::MIN as i128, u8::MAX as i128) {
                                Some(("u8", hex))
                            } else if in_range(
                                min_value,
                                max_value,
                                i8::MIN as i128,
                                i8::MAX as i128,
                            ) {
                                Some(("i8", hex))
                            } else if in_range(
                                min_value,
                                max_value,
                                u16::MIN as i128,
                                u16::MAX as i128 + 1,
                            ) {
                                Some(("u16", hex))
                            } else if in_range(
                                min_value,
                                max_value,
                                i16::MIN as i128,
                                i16::MAX as i128,
                            ) {
                                Some(("i16", hex))
                            } else if in_range(
                                min_value,
                                max_value,
                                u32::MIN as i128,
                                u32::MAX as i128,
                            ) {
                                Some(("u32", hex))
                            } else if in_range(
                                min_value,
                                max_value,
                                i32::MIN as i128,
                                i32::MAX as i128,
                            ) {
                                Some(("i32", hex))
                            } else {
                                None
                            };
                    }
                }
            }
            _ => {}
        }
    }
    let mut field = existing_fields
        .remove(field_name)
        .cloned()
        .unwrap_or_else(|| Field {
            name: Box::from(field_name),
            field_type: None,
            inline_enum: None,
            is_key: false,
            has_auto: false,
            is_set: false,
            is_range_dot: false,
            is_range_dash: false,
            is_optional: false,
            is_read_only: false,
            is_multiple: false,
            is_hex: false,
            reference: Default::default(),
            has_none: false,
            has_unlimited: false,
            has_disabled: false,
            is_rxtx_pair: false,
            keep_if_none: false,
            default: None,
        });
    field.is_read_only = false;
    if let Some((field_type, is_hex)) = number_type {
        field.field_type = Some(Box::from(field_type));
        field.is_hex = is_hex;
    }
    let mut enum_values = Vec::new();
    if !example_values.is_empty() {
        field.has_unlimited = false;
        field.has_auto = false;
        field.has_disabled = false;
        field.has_none = false;
        for example in example_values {
            match example.as_ref() {
                "unlimited" => field.has_unlimited = true,
                "auto" => field.has_auto = true,
                "none" => field.has_none = true,
                "disabled" => field.has_disabled = true,
                _ => {
                    enum_values.push(example);
                }
            }
        }
    }
    //if!symbols.is_empty(){
    //println!("{field_name}: {symbols:?}");
    //}
    if symbols.contains("Interface") {
        if enum_values.is_empty() {
            field.reference = Reference::IsReference("interface".into())
        } else {
            field.reference = Reference::RefereesTo("interface".into())
        }
    }
    if symbols.contains("Rx") {
        field.is_rxtx_pair = true;
    }
    if number_type.is_none() && !enum_values.is_empty() && symbols.is_empty() && !any_value_allowed
    {
        if enum_values.len() == 2
            && enum_values[0].as_ref() == "no"
            && enum_values[1].as_ref() == "yes"
        {
            field.field_type = Some("bool".into());
        } else {
            if has_star {
                println!("{field_name}: Enum with star: {enum_values:?}");
            }
            field.inline_enum = Some(enum_values.into_boxed_slice());
        }
    }
    /*println!(
        "{}",
        to_string_pretty(&field, PrettyConfig::default()).unwrap()
    );*/
    field
}

fn in_range(min_value: i128, max_value: i128, min_range: i128, max_range: i128) -> bool {
    min_value >= min_range
        && min_value <= max_range
        && max_value >= min_range
        && max_value <= max_range
}

fn sentence_of(result: SimpleResult) -> Option<HashMap<Box<str>, Option<Box<str>>>> {
    match result {
        SimpleResult::Sentence(s) => Some(s),
        SimpleResult::Error(e) => {
            error!("Error: {}", e);
            None
        }
        SimpleResult::Trap { category, message } => {
            error!("Trap: {:?} {}", category, message);
            None
        }
    }
}
