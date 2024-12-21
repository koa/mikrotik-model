use convert_case::{Case, Casing};
use proc_macro2::{Ident, Literal, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use syn::__private::ToTokens;
use syn::punctuated::Punctuated;
use syn::{parse_quote, FieldValue, FieldsNamed, Item, Path, PathSegment, Token, Type, TypePath};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Model {
    pub entities: Vec<Entity>,
}
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Entity {
    pub path: Box<[Box<str>]>,
    pub fields: Vec<Field>,
    pub is_single: bool,
}

impl Entity {
    pub fn generate_code(&self) -> impl Iterator<Item=Item> + '_ {
        let struct_name: Box<str> = self
            .path
            .iter()
            .map(|c| c.as_ref().to_case(Case::UpperCamel))
            .collect();
        let path = self.path.join("/");
        let mut fields_named = FieldsNamed {
            brace_token: Default::default(),
            named: Default::default(),
        };
        let mut punctuated_fields: Punctuated<FieldValue, Token![,]> = Punctuated::new();

        let struct_ident = Ident::new(&struct_name, Span::call_site());
        for (field, parser) in self.fields.iter().map(|field| field.generate_code()) {
            fields_named.named.push(field);
            punctuated_fields.push(parser);
        }
        [
            parse_quote! {
                #[derive(Debug, Clone, PartialEq)]
                pub struct #struct_ident #fields_named
            },
            parse_quote! {
                impl resource::RosResource for #struct_ident {
                     fn parse(values: &std::collections::HashMap<String, Option<String>>) -> Result<Self, resource::ResourceAccessError> {
                        Ok(#struct_ident {#punctuated_fields})
                    }
                    fn path()->&'static str{
                        #path
                    }
                }
            },
        ].into_iter()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct Field {
    pub name: Box<str>,
    pub field_type: Option<Box<str>>,
    pub inline_enum: Option<Box<[Box<str>]>>,
    pub is_key: bool,
    pub has_auto: bool,
    pub is_set: bool,
    pub is_range: bool,
    pub is_optional: bool,
    pub is_read_only: bool,
    pub reference: Reference,
}

impl Field {
    pub fn generate_code(&self) -> (syn::Field, FieldValue) {
        let field_name = Ident::new(&self.name.as_ref().to_case(Case::Snake), Span::mixed_site());
        let attribute_name = Literal::string(self.name.as_ref());
        let parsed_field_type = self.field_type.as_ref().map(|field_type| {
            let ident = Ident::new(&field_type, Span::mixed_site());
            let mut segments: Punctuated<PathSegment, Token![::]> = Default::default();
            segments.push(PathSegment::from(ident));
            Type::Path(TypePath {
                qself: None,
                path: Path {
                    leading_colon: None,
                    segments,
                },
            })
        });
        let field_type = parsed_field_type.unwrap_or(parse_quote!(Box<str>));
        //let parse_type = parsed_field_type.unwrap_or(parse_quote!(Box));
        (
            parse_quote!(
                #field_name: #field_type
            ),
            parse_quote! {
                #field_name: values
                    .get(#attribute_name)
                    .and_then(|v| v.as_ref())
                    .map(
                        |value| match value::RosValue::parse_ros(value.as_str()) {
                            value::ParseRosValueResult::None => Err(resource::ResourceAccessError::MissingFieldError {
                                field_name: #attribute_name,
                            }),
                            value::ParseRosValueResult::Value(v) => Ok(v),
                            value::ParseRosValueResult::Invalid => {
                                Err(resource::ResourceAccessError::InvalidValueError {
                                    field_name: #attribute_name,
                                    value: value.clone().into_boxed_str(),
                                })
                            }
                        },
                    )
                    .unwrap_or(Err(resource::ResourceAccessError::MissingFieldError {
                        field_name: #attribute_name,
                    }))?
            },
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct EnumDescriptions(pub HashMap<Box<str>, Box<[Box<str>]>>);

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub enum Reference {
    #[default]
    None,
    IsReference(Box<str>),
    RefereesTo(Box<str>),
}

pub fn parse_lines<'a>(lines: impl Iterator<Item=&'a str>) -> Vec<Entity> {
    let mut collected_entities = Vec::new();
    let mut current_entity = None;
    for line in lines {
        let line = line.split('#').next().unwrap();
        if let Some(name) = line.strip_prefix("/") {
            let path = parse_path(name);
            if let Some(entity) = current_entity.replace(Entity {
                path,
                fields: vec![],
                is_single: false,
            }) {
                collected_entities.push(entity);
            }
        } else if let Some(name) = line.strip_prefix("1/") {
            let path = parse_path(name);
            if let Some(entity) = current_entity.replace(Entity {
                path,
                fields: vec![],
                is_single: true,
            }) {
                collected_entities.push(entity);
            }
        } else if let Some(entity) = current_entity.as_mut() {
            if let Some(field) = parse_field_line(line) {
                entity.fields.push(field);
            }
        }
    }
    if let Some(entity) = current_entity.take() {
        collected_entities.push(entity);
    }
    collected_entities
}

fn parse_path(name: &str) -> Box<[Box<str>]> {
    let path: Box<[Box<str>]> = name
        .trim()
        .split('/')
        .map(|s| s.to_string().into_boxed_str())
        .collect();
    path
}
fn parse_field_line(line: &str) -> Option<Field> {
    if let Some((name, definition)) = line.split_once(':') {
        let mut field = Field {
            name: name.into(),
            ..Field::default()
        };
        for comp in definition.split(';').map(str::trim) {
            if let Some((key, value)) = comp.split_once('=') {
                match key {
                    "enum" => {
                        field.inline_enum =
                            Some(value.split(',').map(|s| s.trim().into()).collect());
                    }
                    "ref" => {}
                    _ => {}
                }
            } else {
                match comp {
                    "id" => field.is_key = true,
                    name => field.field_type = Some(name.into()),
                }
            }
        }
        Some(field)
    } else {
        Some(line.trim())
            .filter(|s| !s.is_empty())
            .map(|name| Field {
                name: name.into(),
                ..Field::default()
            })
    }
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_alias() {
        let ethernet = Entity {
            path: Box::new(["interface".into(), "ethernet".into()]),
            fields: vec![
                Field {
                    name: "default-name".into(),
                    field_type: None,
                    inline_enum: None,
                    is_key: true,
                    has_auto: false,
                    is_set: false,
                    is_range: false,
                    is_optional: false,
                    is_read_only: false,
                    reference: Reference::None,
                },
                Field {
                    name: "name".into(),
                    field_type: None,
                    inline_enum: None,
                    is_key: true,
                    has_auto: false,
                    is_set: false,
                    is_range: false,
                    is_optional: false,
                    is_read_only: false,
                    reference: Reference::IsReference("interface".into()),
                },
                Field {
                    name: "advertise".into(),
                    field_type: Some("EthernetSpeed".into()),
                    inline_enum: None,
                    is_key: false,
                    has_auto: false,
                    is_set: true,
                    is_range: false,
                    is_optional: false,
                    is_read_only: false,
                    reference: Default::default(),
                },
            ],
            is_single: false,
        };
        let model = Model {
            entities: vec![ethernet],
        };
        println!("{}", serde_yaml::to_string(&model).unwrap());
    }

    #[test]
    fn test_entity_parser() {
        let data = include_str!("../../ros_model/system.txt");
        let lines = data.lines();
        let collected_entities = parse_lines(lines);
        println!("{:#?}", collected_entities);
    }
}
