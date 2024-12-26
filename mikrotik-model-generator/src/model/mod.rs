use crate::{cleanup_field_name, derive_ident};
use crate::{generate_enums, KEYWORDS};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, Literal, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use syn::__private::ToTokens;
use syn::punctuated::Punctuated;
use syn::{
    parse_quote, Expr, ExprArray, ExprLit, FieldValue, FieldsNamed, Item, Lit, LitStr, Path,
    PathSegment, Token, Type, TypePath, Variant,
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
    pub fn generate_code(&self) -> impl Iterator<Item=(Item, Option<Variant>)> + '_ {
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
        let mut changed_values_array = ExprArray {
            attrs: vec![],
            bracket_token: Default::default(),
            elems: Default::default(),
        };
        let mut id_fields = Vec::new();
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
            let (field_name, field_type, parser, change_expression) =
                field.generate_code(enum_type);
            let field_def = parse_quote!(
                pub #field_name: #field_type
            );
            if field.is_read_only {
                fields_named_status.named.push(field_def);
                punctuated_fields_status.push(parser.clone());
            } else {
                fields_named_cfg.named.push(field_def);
                punctuated_fields_cfg.push(parser.clone());
                changed_values_array.elems.push(change_expression);
            }
            if field.is_key {
                id_fields.push((field, field_name, field_type, parser));
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
        let has_cfg_struct = !fields_named_cfg.named.is_empty();
        let has_status_struct = !fields_named_status.named.is_empty();
        if has_cfg_struct {
            items.push((
                parse_quote! {
                    #[derive(Debug, Clone, PartialEq)]
                    pub struct #struct_ident_cfg #fields_named_cfg
                },
                Some(parse_quote!(#struct_ident(#struct_ident_cfg))),
            ));
            fields_named.named.push(parse_quote! {
                pub cfg: #struct_ident_cfg
            });
            punctuated_fields.push(parse_quote! {
                cfg: #struct_ident_cfg::parse(values)?
            });
            items.push((parse_quote! {
                impl resource::DeserializeRosResource for #struct_ident_cfg {
                     fn parse(values: &std::collections::HashMap<String, Option<String>>) -> Result<Self, resource::ResourceAccessError> {
                        Ok(#struct_ident_cfg {#punctuated_fields_cfg})
                    }
                    fn path()->&'static str{
                        #path
                    }
                }
            }, None));
            items.push((
                parse_quote! {
                    impl resource::RosResource for #struct_ident_cfg {
                        fn known_fields()->&'static[&'static str]{
                            &#known_fields
                        }
                    }
                },
                None,
            ));
            items.push((
                parse_quote! {
                    impl resource::CfgResource for #struct_ident_cfg {
                        #[allow(clippy::needless_lifetimes)]
                        fn changed_values<'a, 'b>(
                            &'a self,
                            before: &'b Self,
                        ) -> impl Iterator<Item = value::KeyValuePair<'a>> {
                            #changed_values_array.into_iter().flatten()
                        }
                    }
                },
                None,
            ));
            if self.is_single {
                items.push((
                    parse_quote! {
                        impl resource::SingleResource for #struct_ident_cfg {}
                    },
                    None,
                ));
                items.push((
                    parse_quote! {
                        impl resource::Updatable for #struct_ident_cfg {
                            fn calculate_update<'a>(&'a self, from: &'a Self) -> resource::ResourceMutation<'a> {
                                resource::ResourceMutation {
                                    resource: <#struct_ident_cfg as resource::DeserializeRosResource>::path(),
                                    operation: resource::ResourceMutationOperation::UpdateSingle,
                                    fields: resource::CfgResource::changed_values(self,from).collect(),
                                }
                            }
                        }
                    },
                    None,
                ));
            } else {
                let mut plain_items = Vec::new();
                for (id_field, id_field_name, id_field_type, parser) in id_fields {
                    let id_struct_name =
                        cleanup_field_name(&format!("{struct_name}By_{}", id_field.name))
                            .to_case(Case::UpperCamel);
                    let id_struct_ident = Ident::new(&id_struct_name, Span::call_site());
                    let field_name = id_field.name.as_ref();
                    if id_field.is_read_only {
                        plain_items.push(parse_quote! {
                            #[derive(Debug, Clone, PartialEq)]
                            pub struct #id_struct_ident {
                                pub #id_field_name: #id_field_type,
                                pub data: #struct_ident_cfg,
                            }
                        });
                        plain_items.push(parse_quote! {
                                impl resource::DeserializeRosResource for #id_struct_ident {
                                    fn parse(values: &std::collections::HashMap<String, Option<String>>) -> Result<Self, resource::ResourceAccessError> {
                                        Ok(#id_struct_ident{
                                        #parser,
                                        data: <#struct_ident_cfg as resource::DeserializeRosResource>::parse(
                                            values,
                                        )?})
                                    }

                                    fn path() -> &'static str {
                                        #struct_ident_cfg::path()
                                    }
                                }
                            });
                        plain_items.push(parse_quote! {
                            impl resource::KeyedResource for #id_struct_ident {
                                type Key = #id_field_type;

                                fn key_name() -> &'static str {
                                    #field_name
                                }

                                fn key_value(&self) -> &#id_field_type {
                                    &self.#id_field_name
                                }
                            }
                        });
                        plain_items.push(parse_quote! {
                            impl resource::CfgResource for #id_struct_ident {
                                #[allow(clippy::needless_lifetimes)]
                                fn changed_values<'a, 'b>(
                                    &'a self,
                                    before: &'b Self,
                                ) -> impl Iterator<Item = value::KeyValuePair<'a>> {
                                    self.data.changed_values(&before.data)
                                }
                            }
                        });
                    } else {
                        plain_items.push(parse_quote! {
                            #[derive(Debug, Clone, PartialEq)]
                            pub struct #id_struct_ident(pub #struct_ident_cfg);
                        });
                        plain_items.push(parse_quote! {
                                impl resource::DeserializeRosResource for #id_struct_ident {
                                    fn parse(values: &std::collections::HashMap<String, Option<String>>) -> Result<Self, resource::ResourceAccessError> {
                                        Ok(#id_struct_ident(<#struct_ident_cfg as resource::DeserializeRosResource>::parse(
                                            values,
                                        )?))
                                    }

                                    fn path() -> &'static str {
                                        #struct_ident_cfg::path()
                                    }
                                }
                            });
                        plain_items.push(parse_quote! {
                            impl resource::KeyedResource for #id_struct_ident {
                                type Key = #id_field_type;

                                fn key_name() -> &'static str {
                                    #field_name
                                }

                                fn key_value(&self) -> &#id_field_type {
                                    &self.0.#id_field_name
                                }
                            }
                        });
                        plain_items.push(parse_quote! {
                            impl resource::CfgResource for #id_struct_ident {
                                #[allow(clippy::needless_lifetimes)]
                                fn changed_values<'a, 'b>(
                                    &'a self,
                                    before: &'b Self,
                                ) -> impl Iterator<Item = value::KeyValuePair<'a>> {
                                    self.0.changed_values(&before.0)
                                }
                            }
                        });
                    };
                    if has_status_struct {
                        plain_items.push(parse_quote! {
                                impl resource::DeserializeRosResource for (#id_struct_ident, #struct_ident_status) {
                                    fn parse(values: &std::collections::HashMap<String, Option<String>>) -> Result<Self, resource::ResourceAccessError> {
                                        Ok((
                                            #id_struct_ident::parse(values)?,
                                            #struct_ident_status::parse(values)?,
                                        ))
                                    }
                                
                                    fn path()->&'static str{
                                        #path
                                    }
                                }
                            });
                        plain_items.push(parse_quote! {
                                impl resource::KeyedResource for (#id_struct_ident, #struct_ident_status) {
                                    type Key = #id_field_type;

                                    fn key_name() -> &'static str {
                                        #field_name
                                    }

                                    fn key_value(&self) -> &#id_field_type {
                                        self.0.key_value()
                                    }
                                }
                            });
                    }
                }
                for item in plain_items {
                    items.push((item, None));
                }
            }
        };
        if has_status_struct {
            items.push((
                parse_quote! {
                    #[derive(Debug, Clone, PartialEq)]
                    pub struct #struct_ident_status #fields_named_status
                },
                None,
            ));
            items.push((parse_quote! {
                impl resource::DeserializeRosResource for #struct_ident_status {
                     fn parse(values: &std::collections::HashMap<String, Option<String>>) -> Result<Self, resource::ResourceAccessError> {
                        Ok(#struct_ident_status {#punctuated_fields_status})
                    }
                    fn path()->&'static str{
                        #path
                    }
                }
            }, None));

            fields_named.named.push(parse_quote! {
                pub status: #struct_ident_status
            });
            punctuated_fields.push(parse_quote! {
                status: #struct_ident_status::parse(values)?
            });
        }
        if has_cfg_struct && has_status_struct {
            items.push((
                parse_quote! {
                    #[derive(Debug, Clone, PartialEq)]
                    pub struct #struct_ident #fields_named
                },
                None,
            ));
            items.push((
                parse_quote! {
                    impl resource::DeserializeRosResource for #struct_ident {
                        fn parse(values: &std::collections::HashMap<String, Option<String>>) -> Result<Self, resource::ResourceAccessError> {
                            Ok(#struct_ident {#punctuated_fields})
                        }
                        fn path()->&'static str{
                            #path
                        }
                    }
                },
                None,
            ));
            items.push((
                parse_quote! {
                    impl resource::RosResource for #struct_ident {
                        fn known_fields()->&'static[&'static str]{
                            &#known_fields
                        }
                    }
                },
                None,
            ));
        }
        generate_enums(&inline_enums)
            .map(|item| (item, None))
            .collect::<Vec<_>>()
            .into_iter()
            .chain(items)
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
    pub has_unlimited: bool,
    pub has_disabled: bool,
    pub is_rxtx_pair: bool,
}

impl Field {
    fn generate_code(&self, enum_field_type: Option<Type>) -> (Ident, Type, FieldValue, Expr) {
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
        let field_type = if self.has_unlimited {
            parse_quote!(value::HasUnlimited<#field_type>)
        } else {
            field_type
        };
        let field_type = if self.has_none {
            parse_quote!(value::HasNone<#field_type>)
        } else {
            field_type
        };
        let field_type = if self.has_disabled {
            parse_quote!(value::HasDisabled<#field_type>)
        } else {
            field_type
        };
        let field_type = if self.is_rxtx_pair {
            parse_quote!(value::RxTxPair<#field_type>)
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
        let parse_snippet = parse_quote! {
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
        };
        let compare_and_set_snippet = parse_quote! {
           if self.#field_name == before.#field_name {
                None
            } else {
                Some(value::KeyValuePair {
                    key: #attribute_name,
                    value: value::RosValue::encode_ros(&self.#field_name),
                })
            }
        };
        (
            field_name,
            field_type,
            parse_snippet,
            compare_and_set_snippet,
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
                    "unlimited" => field.has_unlimited = true,
                    "rxtxpair" => field.is_rxtx_pair = true,
                    "disabled" => field.has_disabled = true,
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
