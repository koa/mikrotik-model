use crate::model::{parse_lines, EnumDescriptions};
use convert_case::{Case, Casing};
use lazy_static::lazy_static;
use proc_macro2::{Ident, Literal, Span};
use std::collections::BTreeMap;
use std::{collections::HashSet, vec};
use syn::{
    __private::quote::format_ident, parse_quote, punctuated::Punctuated, token::Comma, ExprMatch,
    Item, ItemMod, Variant, Visibility,
};

mod model;
lazy_static! {
    static ref KEYWORDS: HashSet<&'static str> = HashSet::from([
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "Self", "self", "static", "struct", "super", "trait",
        "true", "type", "union", "unsafe", "use", "where", "while", "abstract", "become", "box",
        "do", "final", "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
    ]);
}

pub fn generator() -> syn::File {
    let mut items = vec![
        parse_quote!(
            use crate::{
                resource,
                value::{self, IpOrInterface},
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

    let mut resource_enum_variants: Punctuated<Variant, Comma> = Punctuated::new();
    let mut resource_result_enum_variants: Punctuated<Variant, Comma> = Punctuated::new();
    let mut resource_builder_enum_variants: Punctuated<Variant, Comma> = Punctuated::new();
    let mut resource_init_match: ExprMatch = parse_quote! {match self{}};
    let mut append_field_match: ExprMatch = parse_quote! {match self{}};
    let mut build_match: ExprMatch = parse_quote! {match self{}};

    let mut known_references = BTreeMap::new();

    for content in [
        include_str!("../ros_model/system.txt"),
        include_str!("../ros_model/interface.txt"),
        include_str!("../ros_model/bridge.txt"),
        include_str!("../ros_model/ip.txt"),
        include_str!("../ros_model/ospf.txt"),
    ] {
        let entries = parse_lines(content.lines());

        for entity in entries.iter() {
            let (entity_items, enum_fields, references) = entity.generate_code();
            for item in entity_items {
                items.push(item);
            }
            for field in enum_fields {
                let name = field.name;
                let data_type = field.data;
                let builder_type = field.builder;
                resource_enum_variants.push(parse_quote!(#name));
                resource_result_enum_variants.push(parse_quote!(#name(#data_type)));
                resource_builder_enum_variants.push(parse_quote!(#name(#builder_type)));
                resource_init_match.arms.push(
                    parse_quote! {ResourceType::#name=>ResourceBuilder::#name(Default::default())},
                );
                append_field_match
                    .arms
                    .push(parse_quote! {Self::#name(builder)=><#builder_type as resource::DeserializeRosBuilder<#data_type>>::append_field(builder, key, value)});
                build_match
                    .arms
                    .push(parse_quote! {Self::#name(builder)=>Resource::#name(<#builder_type as resource::DeserializeRosBuilder<#data_type>>::build(builder)?)});
            }
            for entry in references {
                known_references.insert(entry.name, entry.data);
            }
        }
    }
    items.push(parse_quote!(
        #[derive(Copy,Debug,Clone,PartialEq, Hash)]
        pub enum ResourceType {#resource_enum_variants}
    ));
    items.push(parse_quote!(
        #[derive(Debug,Clone,PartialEq)]
        pub enum Resource {#resource_result_enum_variants}
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
        #[derive(Copy,Debug,Clone,PartialEq, Hash)]
        pub enum ReferenceType {#reference_enum_variants}
    ));

    let module = vec![Item::Mod(ItemMod {
        attrs: vec![],
        vis: Visibility::Public(Default::default()),
        unsafety: None,
        mod_token: Default::default(),
        ident: format_ident!("model"),
        content: Some((Default::default(), items)),
        semi: None,
    })];

    syn::File {
        shebang: None,
        attrs: vec![],
        items: module,
    }
}

fn generate_enums<'a, T: Iterator<Item=(Ident, Box<[Box<str>]>)>>(
    enums: T,
) -> impl Iterator<Item=Item> + use < 'a, T > {
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
    name.replace(['.', '/'], "_")
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
    use crate::model::{parse_lines, EnumDescriptions};
    use crate::{generate_enums, generator, name2ident};
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
        let entiries = parse_lines(content.lines());
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
}

fn name2ident(name: &str) -> Ident {
    Ident::new(
        cleanup_field_name(name).to_case(Case::UpperCamel).as_str(),
        Span::call_site(),
    )
}
