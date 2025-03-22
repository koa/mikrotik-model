use crate::model::Entity;
use crate::{cleanup_field_name, name2ident, CONTENT_FILES};
use convert_case::{Case, Casing};
use darling::ast::NestedMeta;
use darling::util::{PathList, SpannedValue};
use darling::{Error, FromMeta};
use proc_macro2::{Ident, TokenStream};
use std::collections::HashMap;
use syn::__private::quote::quote;
use syn::__private::ToTokens;
use syn::spanned::Spanned;
use syn::{
    parse_quote, Block, Expr, ExprStruct, Fields, ItemImpl, ItemStruct, PatTuple, Stmt, TypeTuple,
};

#[cfg(test)]
mod test;

pub fn mikrotik_model(item: TokenStream) -> Result<TokenStream, Error> {
    let mut known_structs = HashMap::new();
    for content in CONTENT_FILES {
        let entries = Entity::parse_lines(content.lines());
        for entity in entries {
            let path = entity
                .path
                .iter()
                .map(|s| s.as_ref())
                .collect::<Vec<&str>>()
                .join("/");
            known_structs.insert(path, entity);
        }
    }
    //println!("attr: {}", item);
    let parameters = NestedMeta::parse_meta_list(item).map_err(Error::from)?;

    //println!("parameters: {:#?}", parameters);
    let params = match MikrotikModelParams::from_list(&parameters) {
        Ok(params) => params,
        Err(e) => return Ok(syn::Error::new(e.span(), e).to_compile_error()),
    };

    let current_struct_name = name2ident(&format!("{}_current", params.name));
    let target_struct_name = name2ident(&format!("{}_target", params.name));

    let mut current_struct: ItemStruct = parse_quote! {
        #[derive(Debug)]
        struct #current_struct_name{}
    };
    let mut current_fetch_init: ExprStruct = parse_quote! {Self{}};
    let mut target_struct: ItemStruct = parse_quote! {struct #target_struct_name{}};
    let mut accumulator = Error::accumulator();
    let mut generate_mutations_expr: Option<Expr> = None;
    match (&mut current_struct.fields, &mut target_struct.fields) {
        (Fields::Named(current_struct_fields), Fields::Named(target_struct_fields)) => {
            for (field, f_type) in params.fields {
                let field_name = Ident::new(
                    cleanup_field_name(field.to_string().as_str())
                        .to_case(Case::Snake)
                        .as_str(),
                    field.span(),
                );
                match f_type {
                    TypeEntry::Single(single_type) => {
                        match known_structs.get(single_type.as_str()) {
                            None => {
                                accumulator.push(
                                    Error::custom("mikrotik path not found")
                                        .with_span(&single_type.span()),
                                );
                            }
                            Some(entry) => {
                                let field_type = entry.struct_type_cfg();
                                if entry.is_single {
                                    current_struct_fields
                                        .named
                                        .push(parse_quote!(#field_name:#field_type));
                                    let not_found_error_msg = format!(
                                        "single value at {} not found",
                                        entry.path.join("/")
                                    );
                                    current_fetch_init.fields.push(parse_quote! {#field_name: <#field_type as mikrotik_model::resource::SingleResource>::fetch(device).await?.expect(#not_found_error_msg)});
                                    generate_mutations_expr = chain(
                                        generate_mutations_expr,
                                        parse_quote! {Some(mikrotik_model::resource::generate_single_update(&from.#field_name,&self.#field_name)).into_iter()},
                                    );
                                    target_struct_fields
                                        .named
                                        .push(parse_quote!(#field_name:#field_type));
                                } else {
                                    accumulator.push(
                                        Error::custom("type is not single")
                                            .with_span(&single_type.span()),
                                    );
                                }
                            }
                        }
                    }
                    TypeEntry::ById { path, keys } => match known_structs.get(path.as_str()) {
                        None => {
                            accumulator.push(
                                Error::custom("mikrotik path not found").with_span(&path.span()),
                            );
                        }
                        Some(entry) => {
                            let field_type = entry.struct_type_cfg();
                            if let Some(id_field) = entry
                                .fields
                                .iter()
                                .find(|f| f.is_key && f.name.as_ref() == ".id")
                            {
                                let mut key_fields = Vec::new();

                                for key_path in keys.iter() {
                                    if let Some(key) = key_path.get_ident() {
                                        let key_string = key.to_string();
                                        if let Some(key_field) = entry
                                            .fields
                                            .iter()
                                            .find(|f| f.name.as_ref() == key_string.as_str())
                                        {
                                            key_fields.push(key_field);
                                        } else {
                                            accumulator.push(
                                                Error::custom(format!(
                                                    "field {} not found",
                                                    key_string.as_str()
                                                ))
                                                .with_span(&key.span()),
                                            );
                                        }
                                    } else {
                                        accumulator.push(
                                            Error::custom("Cannot parse key")
                                                .with_span(&key_path.span()),
                                        );
                                    }
                                }

                                let current_field_type = entry.id_struct_type(id_field);
                                current_struct_fields
                                    .named
                                    .push(parse_quote! {#field_name: Box<[#current_field_type]>});
                                current_fetch_init.fields.push(parse_quote! {#field_name: <#current_field_type as mikrotik_model::resource::KeyedResource>::fetch_all(device).await?});
                                if key_fields.is_empty() {
                                    target_struct_fields
                                        .named
                                        .push(parse_quote!(#field_name:Vec<#field_type>));
                                } else {
                                    let mut key_type: TypeTuple = parse_quote!(());
                                    let mut key_values: PatTuple = PatTuple {
                                        attrs: vec![],
                                        paren_token: Default::default(),
                                        elems: Default::default(),
                                    };
                                    let mut generate_block: Block = parse_quote! {{
                                       let mut entry=entry.clone();
                                    }};

                                    for field in key_fields {
                                        let ty = entry.struct_field_type(field);
                                        key_type.elems.push(ty);
                                        let name = field.generate_field_name();
                                        key_values.elems.push(parse_quote! {#name});
                                        generate_block
                                            .stmts
                                            .push(parse_quote! {entry.#name = #name.clone();});
                                    }
                                    generate_block
                                        .stmts
                                        .push(Stmt::Expr(parse_quote! {entry}, None));

                                    target_struct_fields.named.push(
                                        parse_quote!(#field_name:std::collections::BTreeMap<#key_type,#field_type>),
                                    );
                                    generate_mutations_expr = chain(
                                        generate_mutations_expr,
                                        parse_quote! {
                                            mikrotik_model::resource::generate_add_update_remove_by_id(&from.#field_name,
                                                self.#field_name.iter().map(|(#key_values,entry)|#generate_block).map(std::borrow::Cow::<#field_type>::Owned)
                                            )
                                        },
                                    );
                                }
                            } else {
                                accumulator.push(
                                    Error::custom("type has no .id field").with_span(&path.span()),
                                );
                            }
                        }
                    },
                    TypeEntry::ByKey { .. } => {}
                }
            }
        }
        _ => panic!("Should not be possible"),
    }
    let mut stream = quote! {
        use mikrotik_model::model::*;
        use mikrotik_model::ascii;
    };

    let current_impl: ItemImpl = parse_quote! {
        impl #current_struct_name {
            async fn fetch(device: &MikrotikDevice) -> Result<Self, mikrotik_model::resource::Error> {
                Ok(#current_fetch_init)
            }
        }
    };

    stream.extend(current_struct.to_token_stream());
    stream.extend(current_impl.to_token_stream());
    if let Some(mutations) = generate_mutations_expr {
        stream.extend(target_struct.to_token_stream());
        let target_impl: ItemImpl = parse_quote! {
            impl #target_struct_name {
                fn generate_mutations<'a>(&'a self, from: &'a #current_struct_name)->Result<Box<[mikrotik_model::resource::ResourceMutation<'a>]>, mikrotik_model::resource::ResourceMutationError<'a>> {
                    Ok(#mutations.collect())
                }
            }
        };
        stream.extend(target_impl.to_token_stream());
    }
    accumulator.finish_with(stream)
}

fn chain(chain: Option<Expr>, item: Expr) -> Option<Expr> {
    if let Some(expr_before) = chain {
        Some(parse_quote! {#expr_before.chain(#item)})
    } else {
        Some(parse_quote! {#item})
    }
}
#[derive(FromMeta, Debug)]
struct MikrotikModelParams {
    name: Ident,
    fields: HashMap<Ident, TypeEntry>,
}

#[derive(FromMeta, Debug)]
enum TypeEntry {
    Single(SpannedValue<String>),
    ById {
        path: SpannedValue<String>,
        keys: PathList,
    },
    ByKey {
        path: SpannedValue<String>,
        key: Ident,
    },
}
