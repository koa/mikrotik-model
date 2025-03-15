use crate::model::{Entity, EnumDescriptions};
use convert_case::{Case, Casing};
use lazy_static::lazy_static;
use proc_macro2::{Ident, Literal, Span};
use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::{collections::HashSet, vec};
use syn::__private::ToTokens;
use syn::{
    parse_quote, punctuated::Punctuated, token::Comma, ExprMatch, FieldValue, FieldsNamed, Item,
    Variant,
};

pub mod model;
lazy_static! {
    static ref KEYWORDS: HashSet<&'static str> = HashSet::from([
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "Self", "self", "static", "struct", "super", "trait",
        "true", "type", "union", "unsafe", "use", "where", "while", "abstract", "become", "box",
        "do", "final", "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
    ]);
}

const CONTENT_FILES: [&str; 7] = [
    include_str!("../ros_model/system.txt"),
    include_str!("../ros_model/interface.txt"),
    include_str!("../ros_model/bridge.txt"),
    include_str!("../ros_model/ip.txt"),
    include_str!("../ros_model/ospf.txt"),
    include_str!("../ros_model/interface/Wifi.txt"),
    include_str!("../ros_model/interface/Vxlan.txt"),
];

pub fn known_entities() -> impl Iterator<Item = Entity> {
    CONTENT_FILES
        .into_iter()
        .flat_map(|content| Entity::parse_lines(content.lines()))
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct HashableSet<V: Eq + Hash>(HashSet<V>);
impl<V: Eq + Hash> Hash for HashableSet<V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut value = 0;
        for entry in self.0.iter() {
            let hasher = DefaultHasher::new();
            entry.hash(state);
            value = value ^ hasher.finish();
        }
        state.write_u64(value);
    }
}

pub fn generator() -> syn::File {
    let mut items = vec![
        parse_quote!(
            use crate::{
                resource,
                value::{self, IpOrInterface, ClockFrequency},
                ascii,
            };
        ),
        parse_quote!(
            use std::{time::Duration, net::IpAddr};
        ),
        parse_quote!(
            use mac_address::MacAddress;
        ),
        parse_quote!(
            use ipnet::{IpNet};
        ),
    ];
    let enums: EnumDescriptions =
        serde_yaml::from_str(include_str!("../ros_model/enums.yaml")).unwrap();

    for item in generate_enums(
        enums
            .0
            .into_iter()
            .map(|(key, values)| (name2ident(key.as_ref()), values)),
    ) {
        items.push(item);
    }

    let mut known_references = BTreeMap::new();

    let mut all_generated_types = Vec::new();
    let mut outgoing_chain_edges = HashMap::new();
    let mut entries_by_incoming_references = HashMap::new();

    for content in CONTENT_FILES {
        let entries = Entity::parse_lines(content.lines());

        for entity in entries.iter() {
            let (entity_items, enum_fields, references) = entity.generate_code();
            for item in entity_items {
                items.push(item);
            }
            let mut incoming_references = HashSet::new();
            let mut outgoing_references = HashSet::new();
            for entry in references {
                if entry.incoming {
                    incoming_references.insert(entry.name.clone());
                } else {
                    outgoing_references.insert(entry.name.clone());
                }
                known_references.insert(entry.name, entry.data);
            }

            //incoming_references.retain(|e| !outgoing_references.contains(e));
            for outgoing_ref in outgoing_references.iter() {
                for incoming_ref in incoming_references.iter() {
                    outgoing_chain_edges
                        .entry(outgoing_ref.clone())
                        .or_insert_with(Vec::new)
                        .push(incoming_ref.clone());
                }
            }
            for field in enum_fields {
                all_generated_types.push((
                    field,
                    incoming_references.clone(),
                    outgoing_references.clone(),
                ));
            }
            entries_by_incoming_references
                .entry(HashableSet(incoming_references))
                .or_insert_with(Vec::new)
                .push(entity.path.clone());
        }
    }
    for (references, paths) in entries_by_incoming_references {
        let deps = references
            .0
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        println!("{deps}");
        for path in paths {
            println!(" - {path:?}");
        }
    }

    let path_lengths_of_key = outgoing_chain_edges
        .keys()
        .map(|reference| {
            (
                reference,
                calculate_path_length(reference, &outgoing_chain_edges, &[]),
            )
        })
        .collect::<HashMap<_, _>>();
    all_generated_types.sort_by_key(|(_, refs, _)| {
        refs.iter()
            .map(|r| path_lengths_of_key.get(r).copied().unwrap_or_default() + 1)
            .max()
            .unwrap_or_default()
    });

    let mut resource_enum_variants: Punctuated<Variant, Comma> = Punctuated::new();
    let mut resource_result_enum_variants: Punctuated<Variant, Comma> = Punctuated::new();
    let mut resource_ref_result_enum_variants: Punctuated<Variant, Comma> = Punctuated::new();
    let mut resource_builder_enum_variants: Punctuated<Variant, Comma> = Punctuated::new();
    let mut resource_init_match: ExprMatch = parse_quote! {match self{}};
    let mut append_field_match: ExprMatch = parse_quote! {match self{}};
    let mut build_match: ExprMatch = parse_quote! {match self{}};
    let mut resource2type_match: ExprMatch = parse_quote! {match self{}};
    let mut data_fields = FieldsNamed {
        brace_token: Default::default(),
        named: Default::default(),
    };
    let mut data_loader_fields: Punctuated<FieldValue, Comma> = Punctuated::new();

    for (field, incoming_references, outgoing_references) in all_generated_types {
        let length = incoming_references
            .iter()
            .map(|r| path_lengths_of_key.get(r).copied().unwrap_or_default() + 1)
            .max()
            .unwrap_or_default();
        let name = field.type_name;
        /*println!(
            "{name}: {}",
            incoming_references
                .iter()
                .map(|r| r.to_token_stream().to_string())
                .collect::<Vec<String>>()
                .join(", ")
        );
        println!("{name}: {length}");*/
        let data_type = field.data;
        let builder_type = field.builder;
        resource_enum_variants.push(parse_quote!(#name));
        resource_result_enum_variants.push(parse_quote!(#name(#data_type)));
        resource_ref_result_enum_variants.push(parse_quote!(#name(&'r #data_type)));
        resource_builder_enum_variants.push(parse_quote!(#name(#builder_type)));
        resource_init_match
            .arms
            .push(parse_quote! {ResourceType::#name=>ResourceBuilder::#name(Default::default())});
        append_field_match
            .arms
            .push(parse_quote! {Self::#name(builder)=><#builder_type as resource::DeserializeRosBuilder<#data_type>>::append_field(builder, key, value)});
        build_match
            .arms
            .push(parse_quote! {Self::#name(builder)=>Resource::#name(<#builder_type as resource::DeserializeRosBuilder<#data_type>>::build(builder)?)});
        resource2type_match
            .arms
            .push(parse_quote! {Self::#name(_)=>ResourceType::#name});
        if field.can_update {
            let name = field.field_name;
            data_fields
                .named
                .push(parse_quote!(pub #name: Vec<#data_type>));
            data_loader_fields.push(parse_quote! {#name:crate::util::default_if_missing(<#data_type as resource::KeyedResource>::fetch_all(device).await)?});
        } else if field.can_add {
            let name = field.field_name;
            data_fields
                .named
                .push(parse_quote!(pub #name: Vec<#data_type>));
            data_loader_fields.push(parse_quote! {#name:Default::default()});
        } else if field.is_single {
            let name = field.field_name;
            data_fields.named.push(parse_quote!(pub #name: #data_type));
            data_loader_fields.push(
                parse_quote! {#name:<#data_type as resource::SingleResource>::fetch(device).await?.ok_or(resource::Error::ErrorFetchingSingleItem)?},
            );
        }
    }

    items.push(parse_quote!(
        #[derive(Copy,Debug,Clone,PartialEq, Hash, Eq, enum_iterator::Sequence)]
        pub enum ResourceType {#resource_enum_variants}
    ));
    items.push(parse_quote!(
        #[derive(Debug,Clone,PartialEq)]
        pub enum Resource {#resource_result_enum_variants}
    ));
    items.push(parse_quote!(
        #[derive(Debug,Clone,PartialEq)]
        pub enum ResourceRef<'r> {#resource_ref_result_enum_variants}
    ));
    items.push(parse_quote!(
        #[derive(Debug,Clone,PartialEq)]
        pub enum ResourceBuilder {#resource_builder_enum_variants}
    ));
    items.push(parse_quote! {
        impl ResourceType {
            pub fn create_builder(&self)->ResourceBuilder{
                #resource_init_match
            }
        }
    });
    items.push(parse_quote! {
        impl Resource {
            pub fn type_of(&self)->ResourceType{#resource2type_match}
        }
    });
    items.push(parse_quote!(
        impl ResourceBuilder {
            //type Context=ResourceType;
            pub fn append_field(
                &mut self,
                key: &[u8],
                value: Option<&[u8]>,
            ) -> resource::AppendFieldResult {
                #append_field_match
            }
            pub fn build(self) -> Result<Resource, &'static [u8]> {
                Ok(#build_match)
            }
        }
    ));

    let mut reference_enum_variants: Punctuated<Variant, Comma> = Punctuated::new();
    for (name, _) in known_references {
        reference_enum_variants.push(parse_quote!(#name));
    }
    items.push(parse_quote!(
        #[derive(Copy,Debug,Clone,PartialEq, Hash, Eq)]
        pub enum ReferenceType {#reference_enum_variants}
    ));

    items.push(parse_quote!(
        #[derive(Debug,Clone,PartialEq, Default)]
        pub struct Data #data_fields
    ));
    items.push(parse_quote!(

        impl Data {
            pub async fn fetch_from_device(device: &crate::MikrotikDevice)->Result<Self,resource::Error>{
                Ok(Self{
                    #data_loader_fields
                })
            }
        }
    ));

    /*let module = vec![Item::Mod(ItemMod {
        attrs: vec![],
        vis: Visibility::Public(Default::default()),
        unsafety: None,
        mod_token: Default::default(),
        ident: format_ident!("model"),
        content: Some((Default::default(), items)),
        semi: None,
    })];*/

    syn::File {
        shebang: None,
        attrs: vec![],
        items,
    }
}

fn calculate_path_length(
    key: &Ident,
    chain_links: &HashMap<Ident, Vec<Ident>>,
    current_path: &[&Ident],
) -> usize {
    if current_path.contains(&key) {
        let stream = key.to_token_stream();
        println!(
            "Loop2: {} -> {}",
            current_path
                .iter()
                .map(|v| v.to_token_stream().to_string())
                .collect::<Vec<String>>()
                .join(", "),
            stream
        );
        0
    } else {
        let next_path = [current_path, &[key]].concat();
        let mut current_max = 0;
        if let Some(next_keys) = chain_links.get(key) {
            for next_key in next_keys.iter() {
                let candidate = calculate_path_length(next_key, chain_links, &next_path) + 1;
                if candidate > current_max {
                    current_max = candidate;
                }
            }
        }
        current_max
    }
}

fn generate_enums<'a, T: Iterator<Item = (Ident, Box<[Box<str>]>)>>(
    enums: T,
) -> impl Iterator<Item = Item> + use<'a, T> {
    enums.flat_map(|(name, values)| {
        let mut enum_variants: Punctuated<Variant, Comma> = Punctuated::new();
        let mut parse_match: ExprMatch = parse_quote!(match value {});
        let mut encode_match: ExprMatch = parse_quote!(match self {});
        let mut default_arm =
            parse_quote!(&_ => crate::value::ParseRosValueResult::Invalid,);
        for value in values {
            if let Some(found_type_alias) =
                value.strip_prefix('(').and_then(|v| v.strip_suffix(')'))
            {
                let value_type = Ident::new(found_type_alias, Span::call_site());
                enum_variants.push(parse_quote!(Value(#value_type)));
                default_arm =
                    parse_quote!(value=>#value_type::parse_ros(value).map(#name::Value));
                encode_match
                    .arms
                    .push(parse_quote!(#name::Value(v) => v.encode_ros()))
            } else {
                let ident = Ident::new(&derive_ident(value.as_ref()), Span::call_site());
                let value = Literal::byte_string(value.as_bytes());
                enum_variants.push(parse_quote!(#ident));
                parse_match.arms.push(parse_quote!(#value => crate::value::ParseRosValueResult::Value(#name::#ident),));
                encode_match
                    .arms
                    .push(parse_quote!(#name::#ident => std::borrow::Cow::Borrowed(#value)));
            }
        }
        parse_match.arms.push(default_arm);
        [
            parse_quote! {
                #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
                pub enum #name {
                    #enum_variants
                }
            },
            parse_quote! {
                    impl crate::value::RosValue for #name {
                        fn parse_ros(value: &[u8]) -> crate::value::ParseRosValueResult<Self> {
                            #parse_match
                        }
                        fn encode_ros(&self) -> std::borrow::Cow<[u8]> {
                            #encode_match
                        }
                    }
                }].into_iter()
    }
    )
}
fn cleanup_field_name(name: &str) -> String {
    name.replace(['.', '/', '+'], "_")
}

fn derive_ident(value: &str) -> String {
    let base = cleanup_field_name(value).to_case(Case::UpperCamel);
    if let Some(first_char) = base.chars().next() {
        if first_char.is_numeric() {
            format!("_{base}")
        } else {
            base
        }
    } else {
        base
    }
}

#[cfg(test)]
mod test {
    use crate::model::{Entity, EnumDescriptions};
    use crate::{generate_enums, generator, name2ident, CONTENT_FILES};
    use std::fs::File;
    use std::io::read_to_string;
    use syn::__private::ToTokens;

    #[test]
    fn test_read_enums() {
        let file = File::open("ros_model/enums.yaml").unwrap();
        let enums: EnumDescriptions = serde_yaml::from_reader(&file).unwrap();
        for x in generate_enums(
            enums
                .0
                .into_iter()
                .map(|(key, values)| (name2ident(key.as_ref()), values)),
        ) {
            println!("{}", x.to_token_stream());
        }
    }
    #[test]
    fn test_read_structs() {
        let file = File::open("ros_model/interface.txt").unwrap();
        let content = read_to_string(file).unwrap();
        let entiries = Entity::parse_lines(content.lines());
        let items = entiries.iter().flat_map(|e| e.generate_code().0).collect();
        let f = syn::File {
            shebang: None,
            attrs: vec![],
            items,
        };
        println!("File: \n{}", f.to_token_stream());
    }
    #[test]
    fn test_call_generate() {
        generator();
    }

    #[test]
    fn test_serialize_deserialize() {
        let all_entities: Vec<_> = CONTENT_FILES
            .iter()
            .flat_map(|&content| Entity::parse_lines(content.lines()).into_iter())
            .collect();
        let mut temp_content = String::new();
        for entity in all_entities.iter() {
            entity
                .write_entity_lines(&mut temp_content)
                .expect("Cannot write file");
        }
        let parsed_content = Entity::parse_lines(temp_content.lines());

        /*for (all, parsed) in all_entities.iter().zip(parsed_content.iter()) {
            assert_eq!(all, parsed);
        }*/
        assert_eq!(all_entities, parsed_content);
    }
}

fn name2ident(name: &str) -> Ident {
    Ident::new(
        cleanup_field_name(name).to_case(Case::UpperCamel).as_str(),
        Span::call_site(),
    )
}
fn name2field_ident(name: &str) -> Ident {
    Ident::new(
        cleanup_field_name(name).to_case(Case::Snake).as_str(),
        Span::call_site(),
    )
}
