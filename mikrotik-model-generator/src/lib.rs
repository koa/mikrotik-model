use crate::model::{parse_lines, EnumDescriptions, Model};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, Literal, Span};
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parse_quote, ExprMatch, Item, Variant};

mod model;

pub fn generator() -> syn::File {
    let enums: EnumDescriptions =
        serde_yaml::from_str(include_str!("../ros_model/enums.yaml")).unwrap();
    let content = include_str!("../ros_model/system.txt");
    let entries = parse_lines(content.lines());

    let imports: [Item; 3] = [
        parse_quote!(
            use crate::resource;
        ),
        parse_quote!(
            use crate::value;
        ),
        parse_quote!(
            use std::time::Duration;
        ),
    ];

    syn::File {
        shebang: None,
        attrs: vec![],
        items: imports
            .into_iter()
            .chain(generate_enums(&enums.0))
            .chain(entries.iter().flat_map(|e| e.generate_code()))
            .collect(),
    }
}
fn generate_structs(model: &Model) -> impl Iterator<Item=Item> + use < '_ > {
    model
        .entities
        .iter()
        .flat_map(|entity| entity.generate_code())
}
fn generate_enums(
    enums: &HashMap<Box<str>, Box<[Box<str>]>>,
) -> impl Iterator<Item=Item> + use < '_ > {
    enums.iter().flat_map(|(name, values)| {
        let name = Ident::new(&derive_ident(&name), Span::call_site());
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
                let ident = Ident::new(&derive_ident(value), Span::call_site());
                let value = Literal::string(value);
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
                #[derive(Debug, Copy, Clone, Eq, PartialEq)]
                pub enum #name {
                    #enum_variants
                }
            },
            parse_quote! {
                    impl crate::value::RosValue for #name {
                        fn parse_ros(value: &str) -> crate::value::ParseRosValueResult<Self> {
                            #parse_match
                        }
                        fn encode_ros(&self) -> std::borrow::Cow<str> {
                            #encode_match
                        }
                    }
                }].into_iter()
    }
    )
}
fn derive_ident(value: &str) -> String {
    let base = value.to_case(Case::UpperCamel).replace(['.', ','], "_");
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
    use crate::generate_enums;
    use crate::model::{parse_lines, EnumDescriptions};
    use std::fs::File;
    use std::io::{read_to_string, Read};
    use syn::__private::ToTokens;

    #[test]
    fn test_read_enums() {
        let file = File::open("ros_model/enums.yaml").unwrap();
        let enums: EnumDescriptions = serde_yaml::from_reader(&file).unwrap();
        for x in generate_enums(&enums.0) {
            println!("{}", x.to_token_stream());
        }
    }
    #[test]
    fn test_read_structs() {
        let file = File::open("ros_model/system.txt").unwrap();
        let content = read_to_string(file).unwrap();
        let entiries = parse_lines(content.lines());
        let items = entiries.iter().flat_map(|e| e.generate_code()).collect();
        let f = syn::File {
            shebang: None,
            attrs: vec![],
            items,
        };
        println!("File: \n{}", f.to_token_stream());
    }
}
