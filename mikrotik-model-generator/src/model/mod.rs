use crate::{cleanup_field_name, derive_ident};
use crate::{generate_enums, KEYWORDS};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, Literal, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::{
    parse_quote, Expr, ExprArray, ExprLit, FieldValue, FieldsNamed, Item, Lit, LitStr, Path,
    PathSegment, Token, Type, TypePath,
};

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
        let mut fields_named_status = FieldsNamed {
            brace_token: Default::default(),
            named: Default::default(),
        };
        let mut fields_named_cfg = FieldsNamed {
            brace_token: Default::default(),
            named: Default::default(),
        };
        let mut punctuated_fields_status: Punctuated<FieldValue, Token![,]> = Punctuated::new();
        let mut punctuated_fields_cfg: Punctuated<FieldValue, Token![,]> = Punctuated::new();

        let mut known_fields: ExprArray = ExprArray {
            attrs: vec![],
            bracket_token: Default::default(),
            elems: Default::default(),
        };
        let mut inline_enums = HashMap::new();
        for field in self.fields.iter() {
            let enum_type = if let Some(enum_values) = field.inline_enum.as_ref() {
                let enum_name = format!("{struct_name}_{}", field.name);
                let enum_type = Ident::new(&derive_ident(&enum_name), Span::call_site());
                inline_enums.insert(enum_name.into_boxed_str(), enum_values.clone());
                Some(parse_quote!(#enum_type))
            } else {
                None
            };
            known_fields.elems.push(Expr::Lit(ExprLit {
                attrs: vec![],
                lit: Lit::Str(LitStr::new(&field.name, Span::call_site())),
            }));
            let (field_def, parser) = field.generate_code(enum_type);
            if field.is_read_only {
                fields_named_status.named.push(field_def);
                punctuated_fields_status.push(parser);
            } else {
                fields_named_cfg.named.push(field_def);
                punctuated_fields_cfg.push(parser);
            }
        }
        let struct_ident = Ident::new(&struct_name, Span::call_site());
        let struct_ident_status = Ident::new(&format!("{struct_name}State"), Span::call_site());
        let struct_ident_cfg = Ident::new(&format!("{struct_name}Cfg"), Span::call_site());
        let mut items = Vec::with_capacity(5);
        let mut fields_named = FieldsNamed {
            brace_token: Default::default(),
            named: Default::default(),
        };
        let mut punctuated_fields: Punctuated<FieldValue, Token![,]> = Punctuated::new();
        if !fields_named_cfg.named.is_empty() {
            items.push(parse_quote! {
                #[derive(Debug, Clone, PartialEq)]
                pub struct #struct_ident_cfg #fields_named_cfg
            });
            fields_named.named.push(parse_quote! {
                pub cfg: #struct_ident_cfg
            });
            punctuated_fields.push(parse_quote! {
                cfg: #struct_ident_cfg::parse(values)?
            });
            items.push(parse_quote! {
                impl resource::RosResource for #struct_ident_cfg {
                     fn parse(values: &std::collections::HashMap<String, Option<String>>) -> Result<Self, resource::ResourceAccessError> {
                        Ok(#struct_ident_cfg {#punctuated_fields_cfg})
                    }
                    fn path()->&'static str{
                        #path
                    }
                    fn known_fields()->&'static[&'static str]{
                        &#known_fields
                    }
                }
            });
        };
        if !fields_named_status.named.is_empty() {
            items.push(parse_quote! {
                #[derive(Debug, Clone, PartialEq)]
                pub struct #struct_ident_status #fields_named_status
            });
            fields_named.named.push(parse_quote! {
                pub status: #struct_ident_status
            });
            punctuated_fields.push(parse_quote! {
                status: #struct_ident_status{#punctuated_fields_status}
            });
        }
        items.push(parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #struct_ident #fields_named
        });
        items.push(parse_quote! {
                impl resource::RosResource for #struct_ident {
                     fn parse(values: &std::collections::HashMap<String, Option<String>>) -> Result<Self, resource::ResourceAccessError> {
                        Ok(#struct_ident {#punctuated_fields})
                    }
                    fn path()->&'static str{
                        #path
                    }
                    fn known_fields()->&'static[&'static str]{
                        &#known_fields
                    }
                }
            });

        generate_enums(&inline_enums)
            .collect::<Vec<_>>()
            .into_iter()
            .chain(items.into_iter())
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
    pub is_multiple: bool,
    pub is_hex: bool,
    pub reference: Reference,
    pub has_none: bool,
}

impl Field {
    fn generate_code(&self, enum_field_type: Option<Type>) -> (syn::Field, FieldValue) {
        let field_name = cleanup_field_name(self.name.as_ref()).to_case(Case::Snake);
        let field_name = if KEYWORDS.contains(field_name.as_str()) {
            format!("_{field_name}")
        } else {
            field_name
        };
        let field_name = Ident::new(&field_name, Span::mixed_site());
        let attribute_name = Literal::string(self.name.as_ref());
        let field_type = self
            .field_type
            .as_ref()
            .map(|field_type| {
                let ident = Ident::new(field_type, Span::mixed_site());
                let mut segments: Punctuated<PathSegment, Token![::]> = Default::default();
                segments.push(PathSegment::from(ident));
                Type::Path(TypePath {
                    qself: None,
                    path: Path {
                        leading_colon: None,
                        segments,
                    },
                })
            })
            .or(enum_field_type)
            .unwrap_or(parse_quote!(Box<str>));
        let field_type = if self.is_hex {
            parse_quote!(value::Hex<#field_type>)
        } else {
            field_type
        };

        let field_type = if self.has_auto {
            parse_quote!(value::Auto<#field_type>)
        } else {
            field_type
        };
        let field_type = if self.has_none {
            parse_quote!(value::HasNone<#field_type>)
        } else {
            field_type
        };
        let (field_type, default): (Type, Expr) = if self.is_multiple {
            (
                parse_quote!(std::collections::HashSet<#field_type>),
                parse_quote!(Ok(std::collections::HashSet::new())),
            )
        } else if self.is_optional {
            (parse_quote!(Option<#field_type>), parse_quote!(Ok(None)))
        } else {
            (
                field_type,
                parse_quote!(Err(resource::ResourceAccessError::MissingFieldError {field_name: #attribute_name,})),
            )
        };
        (
            parse_quote!(
                pub #field_name: #field_type
            ),
            parse_quote! {
                #field_name: values
                    .get(#attribute_name)
                    .and_then(|v| v.as_ref())
                    .map(
                        |value| match value::RosValue::parse_ros(value.as_str()) {
                            value::ParseRosValueResult::None => #default,
                            value::ParseRosValueResult::Value(v) => Ok(v),
                            value::ParseRosValueResult::Invalid => {
                                Err(resource::ResourceAccessError::InvalidValueError {
                                    field_name: #attribute_name,
                                    value: value.clone().into_boxed_str(),
                                })
                            }
                        },
                    )
                    .unwrap_or(#default)?
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
                let value = value.trim();
                match key.trim() {
                    "enum" => {
                        field.inline_enum =
                            Some(value.split(',').map(|s| s.trim().into()).collect());
                    }
                    "ref" => {
                        if let Some(name) = value.strip_prefix(">") {
                            field.reference = Reference::RefereesTo(name.trim().into());
                        } else {
                            field.reference = Reference::IsReference(value.into());
                        }
                    }
                    _ => {}
                }
            } else {
                match comp {
                    "id" => field.is_key = true,
                    "ro" => field.is_read_only = true,
                    "auto" => field.has_auto = true,
                    "mu" => field.is_multiple = true,
                    "range" => field.is_range = true,
                    "o" => field.is_optional = true,
                    "hex" => field.is_hex = true,
                    "none" => field.has_none = true,
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
    fn test_entity_parser() {
        let data = include_str!("../../ros_model/system.txt");
        let lines = data.lines();
        let collected_entities = parse_lines(lines);
        println!("{:#?}", collected_entities);
    }
}
