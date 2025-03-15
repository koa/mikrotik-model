use crate::{cleanup_field_name, generate_enums, name2field_ident, KEYWORDS};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, Literal, Span};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use syn::{
    Stmt,
    __private::ToTokens,
    parse_quote,
    punctuated::Punctuated,
    token::{Colon, Comma},
    Block, Expr, ExprArray, ExprField, ExprMatch, ExprStruct, FieldValue, FieldsNamed, FnArg,
    ImplItem, Item, ItemFn, ItemImpl, Member, Path, PathSegment, Token, Type, TypePath,
};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Model {
    pub entities: Vec<Entity>,
}
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Entity {
    pub path: Box<[Box<str>]>,
    pub key_field: Option<Box<str>>,
    pub fields: Vec<Field>,
    pub is_single: bool,
    pub can_add: bool,
    pub no_default: bool,
}

pub struct RosTypeEntry {
    pub type_name: Ident,
    pub field_name: Ident,
    pub builder: Type,
    pub data: Type,
    pub can_update: bool,
    pub can_add: bool,
    pub is_single: bool,
}
pub struct ReferenceEntry {
    pub name: Ident,
    pub field_name: Box<str>,
    pub field_ident: Ident,
    pub incoming: bool,
    pub data: Type,
}

impl Entity {
    pub fn parse_lines<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<Self> {
        let mut collected_entities = Vec::new();
        let mut current_entity = None;
        for line in lines {
            let line = line.split('#').next().unwrap();
            if let Some(name) = line.strip_prefix("/") {
                let entity = if let Some((path, params)) = name.split_once(':') {
                    let path = parse_path(path);
                    let mut entity = Entity {
                        path,
                        key_field: None,
                        fields: vec![],
                        is_single: false,
                        can_add: false,
                        no_default: false,
                    };
                    for param in params.split(';') {
                        if let Some((key, value)) = param.split_once('=') {
                            match key.trim() {
                                "id" => entity.key_field = Some(value.trim().into()),
                                &_ => {
                                    panic!("Unknown param: {param}")
                                }
                            }
                        } else {
                            match param.trim() {
                                "can-add" => entity.can_add = true,
                                "is-single" => entity.is_single = true,
                                "no-default" => entity.no_default = true,
                                "" => {}
                                _ => panic!("Unknown param: {param}"),
                            }
                        }
                    }
                    entity
                } else {
                    let path = parse_path(name);
                    Entity {
                        path,
                        key_field: None,
                        fields: vec![],
                        is_single: false,
                        can_add: false,
                        no_default: false,
                    }
                };
                if let Some(entity) = current_entity.replace(entity) {
                    collected_entities.push(entity);
                }
            } else if let Some(name) = line.strip_prefix("1/") {
                let path = parse_path(name);
                if let Some(entity) = current_entity.replace(Entity {
                    path,
                    key_field: None,
                    fields: vec![],
                    is_single: true,
                    can_add: false,
                    no_default: false,
                }) {
                    collected_entities.push(entity);
                }
            } else if let Some(name) = line.strip_prefix("*/") {
                let path = parse_path(name);
                if let Some(entity) = current_entity.replace(Entity {
                    path,
                    key_field: None,
                    fields: vec![],
                    is_single: false,
                    can_add: true,
                    no_default: false,
                }) {
                    collected_entities.push(entity);
                }
            } else if let Some(entity) = current_entity.as_mut() {
                if let Some(field) = Field::parse_field_line(line) {
                    entity.fields.push(field);
                }
            }
        }
        if let Some(entity) = current_entity.take() {
            collected_entities.push(entity);
        }
        collected_entities
    }
    pub fn write_entity_lines<W: std::fmt::Write>(&self, writer: &mut W) -> std::fmt::Result {
        write!(writer, "/{}:", self.path.join("/"))?;
        if let Some(key) = &self.key_field {
            write!(writer, "id={key};")?;
        }
        if self.can_add {
            write!(writer, "can-add;")?;
        }
        if self.is_single {
            write!(writer, "is-single;")?;
        }
        writer.write_char('\n')?;
        for field in &self.fields {
            field.write_field_line(writer)?;
        }
        writer.write_char('\n')?;
        Ok(())
    }
    pub fn generate_code(
        &self,
    ) -> (
        impl IntoIterator<Item = Item>,
        impl IntoIterator<Item = RosTypeEntry>,
        impl IntoIterator<Item = ReferenceEntry>,
    ) {
        let mut items: Vec<Item> = Vec::with_capacity(50);
        let mut enum_entries: Vec<RosTypeEntry> = Vec::with_capacity(10);
        let references = self.collect_references();

        let has_cfg_struct = self.modifiable_fields_iterator().next().is_some();
        let has_readonly_fields = self.read_only_fields_iterator().next().is_some();

        if has_cfg_struct {
            items.push(self.create_cfg_struct());
            items.push(self.create_cfg_builder_struct());
            enum_entries.push(self.create_cfg_enum_entry());
            items.push(self.generate_has_reference_for_cfg_struct());
            items.push(self.generate_deserialize_for_cfg_struct());
            items.push(self.generate_ros_resource_for_cfg());
            items.push(self.create_deserialize_builder_for_cfg());
            items.push(self.create_cfg_resource());
            if !self.no_default {
                //items.push(self.generate_default_for_cfg());
            }
            let mut cfg_struct_items = Vec::new();
            if self.is_single {
                items.push(self.single_resource_cfg());
                items.push(self.updateable_cfg());
            } else {
                if self.can_add {
                    cfg_struct_items.push(self.create_constructor());
                    items.push(self.create_createable_impl());
                }
                for id_field in self.fields.iter().filter(|f| f.is_key) {
                    items.push(self.generate_has_reference_for_id(id_field));
                    items.push(self.generate_deserialize_for_id(id_field));
                    items.push(self.generate_ros_resource_for_id(id_field));
                    enum_entries.push(self.create_id_enum_entry(id_field));
                    if id_field.is_read_only {
                        items.push(self.generate_id_struct_external(id_field));
                        items.push(self.generate_id_builder_external(id_field));
                        items.push(self.generate_deserialize_builder_for_id_external(id_field));
                        items.push(self.generate_keyed_for_id_external(id_field));
                        items.push(self.generate_cfg_for_id_external(id_field));
                        items.push(self.generate_set_for_id_external(id_field));
                        items.push(self.generate_updateable_id_external(id_field));
                    } else {
                        items.push(self.generate_id_struct_internal(id_field));
                        items.push(self.generate_id_builder_internal(id_field));
                        items.push(self.generate_deserialize_builder_for_id_internal(id_field));
                        items.push(self.generate_keyed_for_id_internal(id_field));
                        items.push(self.generate_cfg_for_id_internal(id_field));
                        items.push(self.generate_updateable_id_internal(id_field));
                        if self.can_add {
                            items.push(self.generate_creatable_id_internal(id_field));
                        }
                    }
                    if has_readonly_fields {
                        items.push(self.generate_has_reference_for_id_combined(id_field));
                        items.push(self.generate_deserialize_for_id_combined(id_field));
                        items.push(self.generate_ros_for_id_combined(id_field));
                        items.push(self.generate_deserialize_builder_for_id_combined(id_field));
                        items.push(self.generate_keyed_for_id_combined(id_field));
                        enum_entries.push(self.create_id_enum_entry_combined(id_field));
                    }
                }
            }
            if !cfg_struct_items.is_empty() {
                let struct_ident_cfg = self.struct_type_cfg();
                let mut impl_item: ItemImpl = parse_quote! {impl #struct_ident_cfg{}};
                impl_item.items.extend(cfg_struct_items);
                items.push(impl_item.into());
            }
        };
        if has_readonly_fields {
            items.push(self.create_status_struct());
            items.push(self.create_status_builder_struct());
            items.push(self.generate_has_reference_for_status_struct());
            items.push(self.generate_deserialize_for_status_struct());
            items.push(self.create_ros_resource_for_status());
            items.push(self.create_deserialize_builder_for_status());
            enum_entries.push(self.create_status_enum_entry());
        }
        if has_cfg_struct && has_readonly_fields {
            enum_entries.push(self.create_enum_entry());
            items.push(self.generate_combined_struct());
            items.push(self.generate_combined_builder_struct());
            items.push(self.generate_has_reference_for_combined_struct());
            items.push(self.generate_deserialize_for_combined_struct());
            items.push(self.generate_combined_deserialize_builder());
            items.push(self.generate_ros_for_combined_struct());
            if self.is_single {
                items.push(self.create_combined_single_impl());
            }
        }
        (
            generate_enums(self.collect_enum_field_types())
                .chain(items)
                .collect::<Vec<_>>(),
            enum_entries,
            references,
        )
    }

    fn collect_enum_field_types(&self) -> impl Iterator<Item = (Ident, Box<[Box<str>]>)> + use<'_> {
        self.fields
            .iter()
            .filter_map(|field| self.enum_field_type(field))
    }

    fn collect_references(&self) -> Box<[ReferenceEntry]> {
        self.referencing_fields()
            .map(|(name, incoming, field)| ReferenceEntry {
                name,
                field_name: field.name.clone(),
                field_ident: field.generate_field_name(),
                incoming,
                data: self.base_field_type(field),
            })
            .collect()
    }

    fn referencing_fields(&self) -> impl Iterator<Item = (Ident, bool, &Field)> {
        self.fields
            .iter()
            .filter_map(|field| match &field.reference {
                Reference::None => None,
                Reference::IsReference(r) => Some((crate::name2ident(r.as_ref()), false, field)),
                Reference::RefereesTo(r) => Some((crate::name2ident(r.as_ref()), true, field)),
            })
    }

    fn create_cfg_builder_struct(&self) -> Item {
        let struct_name = self.struct_ident_cfg_builder();
        let fields = self.modifiable_field_declarations(|f| self.builder_field_type(f));
        parse_quote! {
            #[derive(Debug, Clone, PartialEq, Default)]
            pub struct #struct_name #fields
        }
    }

    fn create_cfg_struct(&self) -> Item {
        let struct_name = self.struct_type_cfg();
        let fields = self.modifiable_field_declarations(|f| self.struct_field_type(f));
        parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #struct_name #fields
        }
    }

    fn generate_ros_for_combined_struct(&self) -> Item {
        let struct_type = self.struct_type();
        let struct_ident = self.struct_ident();
        let path = self.generate_path();
        parse_quote! {
            impl resource::RosResource for #struct_type {
                fn path()->&'static [u8]{
                    #path
                }
                fn create_resource_ref(&self)->ResourceRef{
                    ResourceRef::#struct_ident(self)
                }

                fn provides_reference(&self)->impl Iterator<Item=(ReferenceType, std::borrow::Cow<[u8]>)>{
                    self.cfg.provides_reference().chain(self.status.provides_reference())
                }
                fn consumes_reference(&self)->impl Iterator<Item=(ReferenceType, std::borrow::Cow<[u8]>)>{
                    self.cfg.consumes_reference().chain(self.status.consumes_reference())
                }
            }
        }
    }

    fn generate_deserialize_for_combined_struct(&self) -> Item {
        Self::generate_deserialize(
            self.struct_type(),
            self.struct_builder(),
            self.struct_ident(),
            None,
        )
    }
    fn generate_has_reference_for_combined_struct(&self) -> Item {
        Self::generate_has_reference(
            self.struct_type(),
            Some(parse_quote! {
                fn update_reference<V: value::RosValue>(&mut self,ref_type: ReferenceType,old_value: &V,new_value: &V) -> bool {
                    self.cfg.update_reference(ref_type, old_value, new_value)|
                    self.status.update_reference(ref_type, old_value, new_value)
                }
            }),
        )
    }

    fn generate_combined_struct(&self) -> Item {
        let struct_ident = self.struct_type();
        let struct_ident_cfg = self.struct_type_cfg();
        let struct_ident_status = self.struct_status_type();
        parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #struct_ident {
                pub cfg: #struct_ident_cfg,
                pub status: #struct_ident_status,
            }
        }
    }

    fn generate_keyed_for_id_combined(&self, id_field: &Field) -> Item {
        let id_field_type = self.struct_field_type(id_field);
        let field_name = id_field.attribute_name();
        let struct_ident_status = self.struct_status_type();
        let struct_ident_cfg = self.struct_ident_cfg();
        let id_struct_ident = self.id_struct_type(id_field);
        let item = parse_quote! {
            impl resource::KeyedResource for (#id_struct_ident, #struct_ident_status) {
                type Key = #id_field_type;
                type Value = #struct_ident_cfg;

                fn key_name() -> &'static [u8] {
                    #field_name
                }

                fn key_value(&self) -> &#id_field_type {
                    self.0.key_value()
                }
                fn value(&self) -> &Self::Value{
                    self.0.value()
                }
            }
        };
        item
    }

    fn generate_deserialize_for_id_combined(&self, id_field: &Field) -> Item {
        Self::generate_deserialize(
            self.data_tuples_for_id_combined(id_field),
            self.builder_tuples_for_id_combined(id_field),
            self.id_struct_ident_combined(id_field),
            None,
        )
    }
    fn generate_has_reference_for_id_combined(&self, id_field: &Field) -> Item {
        Self::generate_has_reference(
            self.data_tuples_for_id_combined(id_field),
            Some(parse_quote! {
                fn update_reference<V: value::RosValue>(&mut self,ref_type: ReferenceType,old_value: &V,new_value: &V) -> bool {
                    self.0.update_reference(ref_type, old_value, new_value)|
                    self.1.update_reference(ref_type, old_value, new_value)
                }
            }),
        )
    }

    fn generate_deserialize(
        data_type: Type,
        builder_type: Type,
        variant_name: Ident,
        generate_derived_updates: Option<ItemFn>,
    ) -> Item {
        parse_quote! {
            impl resource::DeserializeRosResource for #data_type {
                type Builder=#builder_type;
                fn unwrap_resource(value: Resource) -> Option<#data_type> {
                    if let Resource::#variant_name(value)=value{
                        Some(value)
                    }else{
                        None
                    }
                }
                fn resource_type()->ResourceType{
                    ResourceType::#variant_name
                }
                #generate_derived_updates
            }
        }
    }
    fn generate_has_reference(data_type: Type, update_reference_fn: Option<ItemFn>) -> Item {
        parse_quote! {
            impl resource::FieldUpdateHandler for #data_type {
                #update_reference_fn
            }
        }
    }

    fn builder_tuples_for_id_combined(&self, id_field: &Field) -> Type {
        let struct_ident_status_builder = self.struct_ident_builder_status();
        let id_struct_ident_builder = self.id_struct_builder_ident(id_field);
        parse_quote! {(#id_struct_ident_builder, #struct_ident_status_builder)}
    }

    fn data_tuples_for_id_combined(&self, id_field: &Field) -> Type {
        let struct_ident_status = self.struct_status_type();
        let id_struct_ident = self.id_struct_type(id_field);
        parse_quote! {(#id_struct_ident, #struct_ident_status)}
    }

    fn generate_cfg_for_id_internal(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        parse_quote! {
            impl resource::CfgResource for #id_struct_ident {
                #[allow(clippy::needless_lifetimes)]
                fn changed_values<'a, 'b>(
                    &'a self,
                    before: &'b Self,
                ) -> impl Iterator<Item = value::KeyValuePair<'a>> {
                    self.0.changed_values(&before.0)
                }
            }
        }
    }
    fn generate_updateable_id_internal(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        let field_name = id_field.attribute_name();
        let id_field_name = id_field.generate_field_name();

        let path = self.generate_path();
        parse_quote! {
            impl resource::Updatable for #id_struct_ident {
                type From=#id_struct_ident;
                fn calculate_update<'a>(&'a self, from: &'a Self) -> resource::ResourceMutation<'a> {
                    resource::ResourceMutation {
                        resource: #path,
                        operation: resource::ResourceMutationOperation::UpdateByKey(value::KeyValuePair {
                            key: #field_name,
                            value: value::RosValue::encode_ros(&from.0.#id_field_name),
                        }),
                        fields: resource::CfgResource::changed_values(self,from).collect(),
                        depends: <#id_struct_ident as resource::RosResource>::consumes_reference(self).filter(|(_,value)|!value.is_empty()).collect(),
                        provides: <#id_struct_ident as resource::RosResource>::provides_reference(self).filter(|(_,value)|!value.is_empty()).collect(),
                    }
                }
            }
        }
    }
    fn generate_creatable_id_internal(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        let path = self.generate_path();
        parse_quote! {
            impl resource::Creatable for #id_struct_ident {
                fn calculate_create<'a>(&'a self) -> resource::ResourceMutation<'a> {
                    self.0.calculate_create()
                }
            }
        }
    }
    fn generate_keyed_for_id_internal(&self, id_field: &Field) -> Item {
        let id_field_type = self.struct_field_type(id_field);
        let id_struct_ident = self.id_struct_type(id_field);
        let struct_ident = self.struct_ident_cfg();
        let field_name = id_field.attribute_name();
        let id_field_name = id_field.generate_field_name();

        let item = parse_quote! {
            impl resource::KeyedResource for #id_struct_ident {
                type Key = #id_field_type;
                type Value=#struct_ident;

                fn key_name() -> &'static [u8] {
                    #field_name
                }

                fn key_value(&self) -> &#id_field_type {
                    &self.0.#id_field_name
                }

                fn value(&self) -> &Self::Value{
                    &self.0
                }
            }
        };
        item
    }

    fn generate_id_struct_internal(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        let struct_ident_cfg = self.struct_type_cfg();

        parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #id_struct_ident(pub #struct_ident_cfg);
        }
    }

    fn generate_cfg_for_id_external(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        parse_quote! {
            impl resource::CfgResource for #id_struct_ident {
                #[allow(clippy::needless_lifetimes)]
                fn changed_values<'a, 'b>(
                    &'a self,
                    before: &'b Self,
                ) -> impl Iterator<Item = value::KeyValuePair<'a>> {
                    self.data.changed_values(&before.data)
                }
            }
        }
    }
    fn generate_set_for_id_external(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        let struct_ident = self.struct_ident_cfg();
        let status_ident = self.struct_status_ident();
        let changed_values_array = self.modifiable_field_updaters(&Some(parse_quote!(data)));

        parse_quote! {
            impl resource::SetResource<#id_struct_ident> for #struct_ident {
                #[allow(clippy::needless_lifetimes)]
                fn changed_values<'a, 'b>(
                    &'a self,
                    before: &'b #id_struct_ident,
                ) -> impl Iterator<Item = value::KeyValuePair<'a>> {
                    #changed_values_array.into_iter().flatten()
                }
            }
        }
    }
    fn generate_updateable_id_external(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        let cfg_ident = self.struct_ident_cfg();
        let path = self.generate_path();
        let key = id_field.attribute_name();
        let key_name = id_field.generate_field_name();
        /*parse_quote! {
            impl resource::Updatable for #id_struct_ident {
                type From=#cfg_ident;
                fn calculate_update<'a>(&'a self, from: &'a #cfg_ident) -> resource::ResourceMutation<'a> {
                    resource::ResourceMutation {
                        resource: #path,
                        operation: resource::ResourceMutationOperation::UpdateByKey(value::KeyValuePair {
                            key: #key,
                            value: value::RosValue::encode_ros(&self.#key_name),
                        }),
                        fields: resource::SetResource::changed_values(from,self).collect(),
                    }
                }
            }
        }*/
        parse_quote! {
            impl resource::Updatable for #id_struct_ident {
                type From=#id_struct_ident;
                fn calculate_update<'a>(&'a self, from: &'a #id_struct_ident) -> resource::ResourceMutation<'a> {
                    resource::ResourceMutation {
                        resource: #path,
                        operation: resource::ResourceMutationOperation::UpdateByKey(value::KeyValuePair {
                            key: #key,
                            value: value::RosValue::encode_ros(&self.#key_name),
                        }),
                        fields: resource::CfgResource::changed_values(self,from).collect(),
                        depends: <#id_struct_ident as resource::RosResource>::consumes_reference(self).filter(|(_,value)|!value.is_empty()).collect(),
                        provides: <#id_struct_ident as resource::RosResource>::provides_reference(self).filter(|(_,value)|!value.is_empty()).collect(),
                    }
                }
            }
        }
    }
    fn generate_keyed_for_id_external(&self, id_field: &Field) -> Item {
        let id_field_type = self.struct_field_type(id_field);
        let field_name = id_field.attribute_name();
        let id_field_name = id_field.generate_field_name();
        let id_struct_ident = self.id_struct_type(id_field);
        let struct_ident = self.struct_ident_cfg();
        let has_dynamic = self.fields.iter().any(|f| {
            f.name.as_ref() == "dynamic"
                && f.field_type.as_deref() == Some("bool")
                && f.is_read_only
        });
        let filter: Option<Stmt> = if has_dynamic {
            Some(parse_quote! {
                fn filter(cmd: mikrotik_api::prelude::CommandBuilder) -> mikrotik_api::prelude::CommandBuilder {
                    cmd.query_equal(b"dynamic",b"false")
                }
            })
        } else {
            None
        };

        parse_quote! {
            impl resource::KeyedResource for #id_struct_ident {
                type Key = #id_field_type;
                type Value = #struct_ident;

                fn key_name() -> &'static [u8] {
                    #field_name
                }

                fn key_value(&self) -> &#id_field_type {
                    &self.#id_field_name
                }

                fn value(&self) -> &Self::Value{
                    &self.data
                }

                #filter
            }
        }
    }

    fn generate_deserialize_for_id(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        let builder_name = self.id_struct_builder_ident(id_field);
        let ident = self.id_struct_ident(id_field);
        let update_fn = if id_field.is_read_only {
            self.generate_derived_updates_body(|f| f == id_field)
                .map(|block| {
                    parse_quote! {
                        fn generate_derived_updates<V: resource::FieldUpdateHandler>(&self,before_value: &Self,handler: &mut V)  {
                            self.data.generate_derived_updates(&before_value.data,handler);
                            #block
                        }
                    }
                }).unwrap_or_else(||
                parse_quote! {
                        fn generate_derived_updates<V: resource::FieldUpdateHandler>(&self,before_value: &Self,handler: &mut V)  {
                            self.data.generate_derived_updates(&before_value.data,handler);
                        }
                    })
        } else {
            parse_quote! {
                fn generate_derived_updates<V: resource::FieldUpdateHandler>(&self,before_value: &Self,handler: &mut V)  {
                    self.0.generate_derived_updates(&before_value.0,handler);
                }
            }
        };
        Self::generate_deserialize(id_struct_ident, builder_name, ident, Some(update_fn))
    }
    fn generate_has_reference_for_id(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        let update_ref_fn = if id_field.is_read_only {
            self
                .update_reference_match_expr(|f| f == id_field)
                .map(|update_reference_match|
                    parse_quote! {
                        fn update_reference<V: value::RosValue>(&mut self, ref_type: ReferenceType, old_value: &V, new_value: &V)->bool{
                            let old_value_any=old_value as &dyn core::any::Any;
                            let new_value_any=new_value as &dyn core::any::Any;
                            let modified = {#update_reference_match};
                            modified | self.data.update_reference(ref_type, old_value, new_value)
                        }
                    }
                ).unwrap_or_else(||
                parse_quote! {
                        fn update_reference<V: value::RosValue>(&mut self,ref_type: ReferenceType,old_value: &V,new_value: &V) -> bool {
                            self.data.update_reference(ref_type, old_value, new_value)
                        }
                    })
        } else {
            parse_quote! {
                fn update_reference<V: value::RosValue>(&mut self,ref_type: ReferenceType,old_value: &V,new_value: &V) -> bool {
                    self.0.update_reference(ref_type, old_value, new_value)
                }
            }
        };
        Self::generate_has_reference(id_struct_ident, Some(update_ref_fn))
    }

    fn generate_id_struct_external(&self, id_field: &Field) -> Item {
        let id_field_type = self.struct_field_type(id_field);
        let struct_ident_cfg = self.struct_type_cfg();
        let id_struct_ident = self.id_struct_type(id_field);
        let id_field_name = id_field.generate_field_name();
        parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #id_struct_ident {
                pub #id_field_name: #id_field_type,
                pub data: #struct_ident_cfg,
            }
        }
    }

    fn id_struct_type(&self, id_field: &Field) -> Type {
        ident2type(self.id_struct_ident(id_field))
    }

    fn id_struct_ident(&self, id_field: &Field) -> Ident {
        let struct_name = self.struct_name();
        crate::name2ident(&format!("{struct_name}By_{}", id_field.name))
    }
    fn id_struct_ident_field(&self, id_field: &Field) -> Ident {
        let struct_name = self.struct_name();
        crate::name2field_ident(&format!("{struct_name}By_{}", id_field.name))
    }

    fn id_struct_builder_ident(&self, id_field: &Field) -> Type {
        let struct_name = self.struct_name();
        name2type(
            cleanup_field_name(&format!("{struct_name}By_{}_Builder", id_field.name))
                .to_case(Case::UpperCamel)
                .as_str(),
        )
    }

    fn generate_deserialize_for_status_struct(&self) -> Item {
        Self::generate_deserialize(
            self.struct_status_type(),
            self.struct_ident_builder_status(),
            self.struct_status_ident(),
            self.generate_derived_updates(|f| f.is_read_only),
        )
    }
    fn generate_has_reference_for_status_struct(&self) -> Item {
        Self::generate_has_reference(
            self.struct_status_type(),
            self.update_reference_fn(|f| f.is_read_only),
        )
    }

    fn create_status_struct(&self) -> Item {
        let fields_named_status = self.create_status_fields(|f| self.struct_field_type(f));
        let struct_ident_status = self.struct_status_type();
        parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #struct_ident_status #fields_named_status
        }
    }

    fn create_createable_impl(&self) -> Item {
        let struct_ident_cfg = self.struct_type_cfg();
        let create_values_array = self.modifiable_field_creators();
        let item = parse_quote! {
            impl resource::Creatable for #struct_ident_cfg{
                fn calculate_create(&self) -> resource::ResourceMutation<'_> {
                    resource::ResourceMutation {
                        resource: <#struct_ident_cfg as resource::RosResource>::path(),
                        operation: resource::ResourceMutationOperation::Add,
                        fields: #create_values_array.into_iter().filter(|value::KeyValuePair{key:_,value}|!value.is_empty()).collect(),
                        depends: <#struct_ident_cfg as resource::RosResource>::consumes_reference(self).filter(|(_,value)|!value.is_empty()).collect(),
                        provides: <#struct_ident_cfg as resource::RosResource>::provides_reference(self).filter(|(_,value)|!value.is_empty()).collect(),
                    }
                }
            }
        };
        item
    }

    fn create_constructor(&self) -> ImplItem {
        let mut params: Punctuated<FnArg, Comma> = Default::default();
        let mut fields_named: Punctuated<FieldValue, Token![,]> = Punctuated::new();

        for field in self.modifiable_fields_iterator() {
            let field_name = field.generate_field_name();
            let field_type = self.struct_field_type(field);
            if field.is_optional || field.is_multiple {
                fields_named.push(FieldValue {
                    attrs: vec![],
                    member: Member::Named(field_name),
                    colon_token: Some(Colon::default()),
                    expr: parse_quote!(Default::default()),
                });
            } else {
                params.push(parse_quote!(#field_name: #field_type));
                fields_named.push(parse_quote!(#field_name));
            }
        }
        parse_quote! {
            pub fn new(#params)->Self{
                Self{
                    #fields_named
                }
            }
        }
    }

    fn updateable_cfg(&self) -> Item {
        let struct_ident_cfg = self.struct_type_cfg();
        parse_quote! {
            impl resource::Updatable for #struct_ident_cfg {
                type From = #struct_ident_cfg;
                fn calculate_update<'a>(&'a self, from: &'a Self) -> resource::ResourceMutation<'a> {
                    resource::ResourceMutation {
                        resource: <#struct_ident_cfg as resource::RosResource>::path(),
                        operation: resource::ResourceMutationOperation::UpdateSingle,
                        fields: resource::CfgResource::changed_values(self,from).collect(),
                        depends: <#struct_ident_cfg as resource::RosResource>::consumes_reference(self).filter(|(_,value)|!value.is_empty()).collect(),
                        provides: <#struct_ident_cfg as resource::RosResource>::provides_reference(self).filter(|(_,value)|!value.is_empty()).collect(),
                    }
                }
            }
        }
    }

    fn single_resource_cfg(&self) -> Item {
        let struct_ident_cfg = self.struct_type_cfg();
        parse_quote! {
            impl resource::SingleResource for #struct_ident_cfg {}
        }
    }

    fn create_cfg_resource(&self) -> Item {
        let struct_ident_cfg = self.struct_type_cfg();
        let changed_values_array = self.modifiable_field_updaters(&None);
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
        }
    }

    fn generate_ros_resource_for_cfg(&self) -> Item {
        self.generate_ros_resource(
            self.struct_type_cfg(),
            self.struct_ident_cfg(),
            self.generate_path(),
            |field| {
                if field.is_read_only {
                    None
                } else {
                    let ident = field.generate_field_name();
                    Some(parse_quote!(&self.#ident))
                }
            },
        )
    }

    fn generate_ros_resource<FG: Fn(&Field) -> Option<Expr>>(
        &self,
        struct_ident_cfg: Type,
        resource_enum_ident: Ident,
        path: Literal,
        field_gen: FG,
    ) -> Item {
        let (provides_array, consumes_array) = self.consume_and_provides(field_gen);
        parse_quote! {
            impl resource::RosResource for #struct_ident_cfg {
                fn path()->&'static [u8]{
                    #path
                }
                fn create_resource_ref(&self)->ResourceRef{
                    ResourceRef::#resource_enum_ident(self)
                }

                fn provides_reference(&self)->impl Iterator<Item=(ReferenceType, std::borrow::Cow<[u8]>)>{
                    #provides_array.into_iter().filter(|(_,value):&(ReferenceType,std::borrow::Cow<[u8]>)|!value.is_empty())
                }
                fn consumes_reference(&self)->impl Iterator<Item=(ReferenceType, std::borrow::Cow<[u8]>)>{
                    #consumes_array.into_iter().filter(|(_,value):&(ReferenceType,std::borrow::Cow<[u8]>)|!value.is_empty())
                }
            }
        }
    }

    fn consume_and_provides<FG: Fn(&Field) -> Option<Expr>>(
        &self,
        field_gen: FG,
    ) -> (ExprArray, ExprArray) {
        let mut provides_array: ExprArray = parse_quote!([]);
        let mut consumes_array: ExprArray = parse_quote!([]);
        for (name, incoming, field) in self.referencing_fields() {
            if let Some(field_access) = field_gen(field) {
                let expr =
                    parse_quote!((ReferenceType::#name,value::RosValue::encode_ros(#field_access)));
                if incoming {
                    consumes_array.elems.push(expr);
                } else {
                    provides_array.elems.push(expr);
                }
            }
        }
        (provides_array, consumes_array)
    }

    fn generate_path(&self) -> Literal {
        Literal::byte_string(self.path.join("/").as_bytes())
    }

    fn generate_deserialize_for_cfg_struct(&self) -> Item {
        Self::generate_deserialize(
            self.struct_type_cfg(),
            self.struct_ident_cfg_builder(),
            self.struct_ident_cfg(),
            self.generate_derived_updates(|f| !f.is_read_only),
        )
    }

    fn update_reference_fn(&self, field_filter: impl Fn(&Field) -> bool) -> Option<ItemFn> {
        self.update_reference_match_expr(field_filter).map(|update_reference_match|
            parse_quote! {
                fn update_reference<V: value::RosValue>(&mut self, ref_type: ReferenceType, old_value: &V, new_value: &V)->bool{
                    let old_value_any=old_value as &dyn core::any::Any;
                    let new_value_any=new_value as &dyn core::any::Any;
                    #update_reference_match
                }
            }
        )
    }
    fn generate_derived_updates(&self, field_filter: impl Fn(&Field) -> bool) -> Option<ItemFn> {
        self.generate_derived_updates_body(field_filter).map(|body|
            parse_quote! {
                fn generate_derived_updates<V: resource::FieldUpdateHandler>(&self, before_value: &Self, handler: &mut V) #body
            }
        )
    }
    fn generate_derived_updates_body(
        &self,
        field_filter: impl Fn(&Field) -> bool + Sized,
    ) -> Option<Block> {
        let mut body: Block = parse_quote!({});
        for (reference_name, field) in
            self.referencing_fields()
                .filter_map(|(reference_ident, incoming, field)| {
                    if !incoming && field_filter(field) {
                        Some((reference_ident, field))
                    } else {
                        None
                    }
                })
        {
            let field_name = field.generate_field_name();
            body.stmts.push(parse_quote! {
                if self.#field_name != before_value.#field_name{
                    handler.update_reference(ReferenceType::#reference_name, &before_value.#field_name, &self.#field_name);
                }
            });
        }
        Some(body).filter(|b| !b.stmts.is_empty())
    }

    fn update_reference_match_expr(
        &self,
        field_filter: impl Fn(&Field) -> bool + Sized,
    ) -> Option<ExprMatch> {
        let mut fields_of_type = BTreeMap::new();
        for (reference_ident, _, field) in self
            .referencing_fields()
            .filter(|(_, _, f)| field_filter(f))
        {
            let base_type = self.base_field_type(field);
            fields_of_type
                .entry(reference_ident)
                .or_insert_with(HashMap::new)
                .entry(base_type.to_token_stream().to_string())
                .or_insert_with(|| (base_type, Vec::new()))
                .1
                .push(field);
        }
        let update_reference_fn: Option<ExprMatch> = if fields_of_type.is_empty() {
            None
        } else {
            let mut update_reference_match: ExprMatch = parse_quote! {match ref_type{}};
            for (reference_type, entries_of_type) in fields_of_type {
                let mut reference_block: Block = parse_quote!({
                    let mut modified = false;
                });
                for (base_type, fields_of_type) in entries_of_type.values() {
                    let mut type_block: Block = parse_quote!({});

                    for field in fields_of_type {
                        let field_name = field.generate_field_name();
                        if field.is_multiple {
                            if field.is_optional {
                                type_block.stmts.push(parse_quote! {
                                    if let Some(values)=self.#field_name.as_mut(){
                                        if values.remove(old_value) {
                                            values.insert(new_value.clone());
                                            modified = true;
                                        }
                                    }
                                });
                            } else {
                                type_block.stmts.push(parse_quote! {
                                        if self.#field_name.remove(old_value) {
                                            self.#field_name.insert(new_value.clone());
                                            modified = true;
                                        }
                                });
                            }
                        } else if field.is_optional {
                            type_block.stmts.push(parse_quote! {
                                    if Some(old_value)== self.#field_name.as_ref() {
                                        self.#field_name = Some(new_value.clone());
                                        modified = true;
                                    }
                            });
                        } else {
                            type_block.stmts.push(parse_quote! {
                                    if old_value== &self.#field_name {
                                        self.#field_name = new_value.clone();
                                        modified = true;
                                    }
                            });
                        }
                    }
                    reference_block.stmts.push(parse_quote! {
                        if let (Some(old_value),Some(new_value)) = (old_value_any.downcast_ref::<#base_type>(),new_value_any.downcast_ref::<#base_type>())
                            #type_block
                    })
                }
                let return_statement = Stmt::Expr(parse_quote!(modified), None);
                reference_block.stmts.push(return_statement);
                update_reference_match
                    .arms
                    .push(parse_quote!(ReferenceType::#reference_type => #reference_block));
            }
            update_reference_match.arms.push(parse_quote!(_ => false));
            Some(update_reference_match)
        };
        update_reference_fn
    }

    fn create_cfg_enum_entry(&self) -> RosTypeEntry {
        let type_name = self.struct_ident_cfg();
        let builder = self.struct_ident_cfg_builder();
        let data = self.struct_type_cfg();
        let field_name = self.struct_ident_cfg_field();

        RosTypeEntry {
            type_name,
            builder,
            data,
            can_update: false,
            can_add: self.can_add,
            is_single: self.is_single,
            field_name,
        }
    }
    fn create_status_enum_entry(&self) -> RosTypeEntry {
        let type_name = self.struct_status_ident();
        let builder = self.struct_ident_builder_status();
        let data = self.struct_status_type();
        let field_name = self.struct_status_ident_field();
        RosTypeEntry {
            type_name,
            field_name,
            builder,
            data,
            can_update: false,
            can_add: false,
            is_single: false,
        }
    }
    fn create_enum_entry(&self) -> RosTypeEntry {
        let type_name = self.struct_ident();
        let builder = self.struct_builder();
        let data = self.struct_type();
        RosTypeEntry {
            type_name,
            field_name: name2field_ident(&self.struct_name()),
            builder,
            data,
            can_update: false,
            can_add: false,
            is_single: false,
        }
    }

    fn modifiable_field_creators(&self) -> ExprArray {
        let mut create_values_array = ExprArray {
            attrs: vec![],
            bracket_token: Default::default(),
            elems: Default::default(),
        };
        for field in self.modifiable_fields_iterator() {
            let field_key = Literal::byte_string(field.name.as_bytes());
            let field_name = field.generate_field_name();
            let field_type = self.struct_field_type(field);
            create_values_array.elems.push(parse_quote! {
                value::KeyValuePair {
                    key: #field_key,
                    value: <#field_type as value::RosValue>::encode_ros(&self.#field_name),
                }
            });
        }
        create_values_array
    }

    fn modifiable_field_updaters(&self, sub_field: &Option<Ident>) -> ExprArray {
        let mut changed_values_array = ExprArray {
            attrs: vec![],
            bracket_token: Default::default(),
            elems: Default::default(),
        };
        for field in self.modifiable_fields_iterator() {
            changed_values_array
                .elems
                .push(field.compare_and_set_snippet(sub_field));
        }
        changed_values_array
    }

    fn modifiable_field_declarations(&self, type_builder: impl Fn(&Field) -> Type) -> FieldsNamed {
        let mut fields_named_cfg = FieldsNamed {
            brace_token: Default::default(),
            named: Default::default(),
        };
        for f in self.modifiable_fields_iterator() {
            let field_name = f.generate_field_name();
            let field_type = type_builder(f);
            fields_named_cfg
                .named
                .push(parse_quote!(pub #field_name: #field_type));
        }
        fields_named_cfg
    }

    fn modifiable_fields_iterator(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|f| !f.is_read_only)
    }

    fn create_status_fields(&self, type_builder: impl Fn(&Field) -> Type) -> FieldsNamed {
        let mut fields_named_status = FieldsNamed {
            brace_token: Default::default(),
            named: Default::default(),
        };
        for field in self.read_only_fields_iterator() {
            let field_name = field.generate_field_name();
            let field_type = type_builder(field);
            let field_def = parse_quote!(
                pub #field_name: #field_type
            );
            fields_named_status.named.push(field_def);
        }
        fields_named_status
    }

    fn read_only_fields_iterator(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|f| f.is_read_only)
    }

    fn builder_field_type(&self, field: &Field) -> Type {
        field.generate_builder_type(self.enum_field_type(field).map(|(ty, _)| ident2type(ty)))
    }

    fn struct_field_type(&self, field: &Field) -> Type {
        field.generate_struct_field_type(self.enum_field_type(field).map(|(ty, _)| ident2type(ty)))
    }
    fn base_field_type(&self, field: &Field) -> Type {
        field.generate_base_field_type(self.enum_field_type(field).map(|(ty, _)| ident2type(ty)))
    }

    fn enum_field_type(&self, field: &Field) -> Option<(Ident, Box<[Box<str>]>)> {
        if let Some(enum_values) = field.inline_enum.as_ref() {
            let struct_name = self.struct_name();
            let enum_name = format!("{struct_name}_{}", field.name);
            let enum_type = crate::name2ident(&enum_name);
            Some((enum_type, enum_values.clone()))
        } else {
            None
        }
    }

    fn struct_type_cfg(&self) -> Type {
        ident2type(self.struct_ident_cfg())
    }

    fn struct_ident_cfg(&self) -> Ident {
        let struct_name = self.struct_name();
        crate::name2ident(&format!("{struct_name}Cfg"))
    }
    fn struct_ident_cfg_field(&self) -> Ident {
        let struct_name = self.struct_name();
        crate::name2field_ident(&format!("{struct_name}Cfg"))
    }

    fn struct_ident_cfg_builder(&self) -> Type {
        let struct_name = self.struct_name();
        let ident = Ident::new(&format!("{struct_name}CfgBuilder"), Span::call_site());
        ident2type(ident)
    }

    fn struct_status_type(&self) -> Type {
        ident2type(self.struct_status_ident())
    }

    fn struct_status_ident(&self) -> Ident {
        let struct_name = self.struct_name();
        crate::name2ident(&format!("{struct_name}State"))
    }
    fn struct_status_ident_field(&self) -> Ident {
        let struct_name = self.struct_name();
        crate::name2field_ident(&format!("{struct_name}State"))
    }

    fn struct_ident_builder_status(&self) -> Type {
        let struct_name = self.struct_name();
        name2type(&format!("{struct_name}StateBuilder"))
    }

    fn struct_type(&self) -> Type {
        ident2type(self.struct_ident())
    }

    fn struct_ident(&self) -> Ident {
        crate::name2ident(&self.struct_name())
    }

    fn struct_builder(&self) -> Type {
        let struct_name = self.struct_name();
        name2type(&format!("{struct_name}Builder"))
    }

    fn struct_name(&self) -> Box<str> {
        self.path
            .iter()
            .map(|c| c.as_ref().to_case(Case::UpperCamel))
            .collect()
    }

    fn create_deserialize_builder_for_cfg(&self) -> Item {
        Self::generate_deserialize_for_builder(
            self.struct_ident_cfg_builder(),
            self.struct_type_cfg(),
            || self.fields.iter().filter(|f| !f.is_read_only),
        )
    }

    fn generate_deserialize_for_builder<'a, F: Fn() -> I, I: Iterator<Item = &'a Field>>(
        builder_type: Type,
        ty: Type,
        fields: F,
    ) -> Item {
        let mut append_field_match: ExprMatch = parse_quote! {
            match (key, value.as_ref()){
            }
        };
        for field in fields() {
            let field_name = field.generate_field_name();
            let attribute_name = field.attribute_name();
            let ok_expression: Expr = if field.is_optional || field.is_multiple {
                parse_quote!(v)
            } else {
                parse_quote!(Some(v))
            };
            append_field_match.arms.push(parse_quote! {
                (#attribute_name, Some(&value)) => match value::RosValue::parse_ros(value) {
                    value::ParseRosValueResult::None =>  {
                        self.#field_name = Default::default();
                        resource::AppendFieldResult::Appended
                    }
                    value::ParseRosValueResult::Value(v) => {
                        self.#field_name = #ok_expression;
                        resource::AppendFieldResult::Appended
                    }
                    value::ParseRosValueResult::Invalid => {
                        resource::AppendFieldResult::InvalidValue(#attribute_name)
                    }
                }
            })
        }
        append_field_match
            .arms
            .push(parse_quote!(_ => resource::AppendFieldResult::UnknownField));

        let mut init_struct: ExprStruct = parse_quote!(#ty{});
        for field in fields() {
            let field_name = field.generate_field_name();
            if field.is_optional || field.is_multiple {
                init_struct
                    .fields
                    .push(parse_quote!(#field_name: self.#field_name));
            } else {
                let attribute_name = field.attribute_name();
                init_struct.fields.push(
                    parse_quote!(#field_name: self.#field_name.ok_or(#attribute_name as &[u8])?),
                );
            }
        }
        parse_quote! {
            impl resource::DeserializeRosBuilder<#ty> for #builder_type {
                type Context=();
                fn init(_ctx: &Self::Context)->Self{
                    Self::default()
                }
                fn append_field(&mut self, key: &[u8], value: Option<&[u8]>) -> resource::AppendFieldResult {
                    #append_field_match
                }
                fn build(self)->Result<#ty,&'static [u8]>{
                    Ok(#init_struct)
                }
            }
        }
    }

    fn generate_id_builder_external(&self, id_field: &Field) -> Item {
        let id_builder_type = self.builder_field_type(id_field);
        let field_name = id_field.generate_field_name();
        let builder_name = self.id_struct_builder_ident(id_field);
        let data_name = self.struct_ident_cfg_builder();
        parse_quote! {
            #[derive(Debug, Clone, PartialEq, Default)]
            pub struct #builder_name{
                pub #field_name: #id_builder_type,
                pub data: #data_name,
            }
        }
    }

    fn generate_deserialize_builder_for_id_external(&self, id_field: &Field) -> Item {
        let field_name = id_field.generate_field_name();
        let attribute_name = id_field.attribute_name();
        let builder_name = self.id_struct_builder_ident(id_field);
        let struct_name = self.id_struct_type(id_field);
        let ok_expression: Expr = if id_field.is_optional || id_field.is_multiple {
            parse_quote!(v)
        } else {
            parse_quote!(Some(v))
        };
        let id_builder_value: Expr = if id_field.is_optional || id_field.is_multiple {
            parse_quote!(self.#field_name)
        } else {
            parse_quote!(self.#field_name.ok_or(#attribute_name as &[u8])?)
        };

        parse_quote! {
            impl resource::DeserializeRosBuilder<#struct_name> for #builder_name {
                type Context=();
                fn init(_ctx: &Self::Context)->Self{
                    Self::default()
                }
                fn append_field(&mut self, key: &[u8], value: Option<&[u8]>) -> resource::AppendFieldResult {
                     match (key, value.as_ref()) {
                        (#attribute_name, Some(&value)) => match value::RosValue::parse_ros(value) {
                            value::ParseRosValueResult::None => {
                                resource::AppendFieldResult::InvalidValue(#attribute_name)
                            }
                            value::ParseRosValueResult::Value(v) => {
                                self.#field_name = #ok_expression;
                                resource::AppendFieldResult::Appended
                            }
                            value::ParseRosValueResult::Invalid => {
                                resource::AppendFieldResult::InvalidValue(#attribute_name)
                            }
                        },
                        (key, value) => self.data.append_field(key, value.copied()),
                    }
                }
                fn build(self)->Result<#struct_name,&'static [u8]>{
                    Ok(#struct_name{
                        #field_name: #id_builder_value,
                        data: self.data.build()?
                    })
                }

            }
        }
    }

    fn generate_deserialize_builder_for_id_internal(&self, id_field: &Field) -> Item {
        let builder_name = self.id_struct_builder_ident(id_field);
        let struct_name = self.id_struct_type(id_field);
        parse_quote! {
            impl resource::DeserializeRosBuilder<#struct_name> for #builder_name {
                type Context=();
                fn init(_ctx: &Self::Context)->Self{
                    Self::default()
                }
                fn append_field(&mut self, key: &[u8], value: Option<&[u8]>) -> resource::AppendFieldResult {
                    self.0.append_field(key, value)
                }
                fn build(self)->Result<#struct_name,&'static [u8]>{
                    Ok(#struct_name(self.0.build()?))
                }

            }
        }
    }

    fn generate_id_builder_internal(&self, id_field: &Field) -> Item {
        let builder_name = self.id_struct_builder_ident(id_field);
        let data_name = self.struct_ident_cfg_builder();
        parse_quote! {
            #[derive(Debug, Clone, PartialEq, Default)]
            pub struct #builder_name(pub #data_name);
        }
    }

    fn create_status_builder_struct(&self) -> Item {
        let struct_name = self.struct_ident_builder_status();
        let fields = self.create_status_fields(|f| self.builder_field_type(f));
        parse_quote! {
            #[derive(Debug, Clone, PartialEq, Default)]
            pub struct #struct_name #fields
        }
    }

    fn create_ros_resource_for_status(&self) -> Item {
        let struct_ident_status = self.struct_status_type();
        let ident = self.struct_status_ident();
        let path = self.generate_path();
        let (provides_array, consumes_array) = self.consume_and_provides(|field| {
            if field.is_read_only {
                let name = field.generate_field_name();
                Some(parse_quote! {&self.#name})
            } else {
                None
            }
        });
        parse_quote! {
            impl resource::RosResource for #struct_ident_status {
                fn path()->&'static [u8]{
                    #path
                }
                fn create_resource_ref(&self)->ResourceRef{
                    ResourceRef::#ident(self)
                }
                fn provides_reference(&self)->impl Iterator<Item=(ReferenceType, std::borrow::Cow<[u8]>)>{
                    #provides_array.into_iter()
                }
                fn consumes_reference(&self)->impl Iterator<Item=(ReferenceType, std::borrow::Cow<[u8]>)>{
                    #consumes_array.into_iter()
                }

            }
        }
    }

    fn create_deserialize_builder_for_status(&self) -> Item {
        Self::generate_deserialize_for_builder(
            self.struct_ident_builder_status(),
            self.struct_status_type(),
            || self.fields.iter().filter(|f| f.is_read_only),
        )
    }

    fn generate_deserialize_builder_for_id_combined(&self, id_field: &Field) -> Item {
        let builder_type = self.builder_tuples_for_id_combined(id_field);
        let data_type = self.data_tuples_for_id_combined(id_field);
        parse_quote! {
            impl resource::DeserializeRosBuilder<#data_type> for #builder_type {
                type Context=();
                fn init(_ctx: &Self::Context)->Self{
                    Self::default()
                }
                fn append_field(&mut self, key: &[u8], value: Option<&[u8]>) -> resource::AppendFieldResult {
                    match (self.0.append_field(key, value),self.1.append_field(key, value)) {
                       (resource::AppendFieldResult::InvalidValue(v),_)|(_,resource::AppendFieldResult::InvalidValue(v))=>resource::AppendFieldResult::InvalidValue(v),
                       (resource::AppendFieldResult::Appended,_)|(_,resource::AppendFieldResult::Appended)=>resource::AppendFieldResult::Appended,
                        _ => resource::AppendFieldResult::UnknownField,
                    }
                }
                fn build(self)->Result<#data_type,&'static [u8]>{
                    Ok((self.0.build()?, self.1.build()?))
                }
            }
        }
    }

    fn generate_ros_resource_for_id(&self, id_field: &Field) -> Item {
        let read_only_id = id_field.is_read_only;
        self.generate_ros_resource(
            self.id_struct_type(id_field),
            self.id_struct_ident(id_field),
            self.generate_path(),
            |field| {
                let ident = field.generate_field_name();
                if read_only_id {
                    if field == id_field {
                        Some(parse_quote!(&self.data_id.#ident))
                    } else {
                        if field.is_read_only {
                            None
                        } else {
                            Some(parse_quote!(&self.data.#ident))
                        }
                    }
                } else {
                    if field.is_read_only {
                        None
                    } else {
                        Some(parse_quote!(&self.0.#ident))
                    }
                }
            },
        )
    }

    fn generate_ros_for_id_combined(&self, id_field: &Field) -> Item {
        let read_only_id = id_field.is_read_only;
        let data_type = self.data_tuples_for_id_combined(id_field);
        self.generate_ros_resource(
            data_type,
            self.id_struct_ident_combined(id_field),
            self.generate_path(),
            |field| {
                let ident = field.generate_field_name();
                if field.is_read_only {
                    Some(parse_quote!(&self.1.#ident))
                } else {
                    if read_only_id {
                        Some(parse_quote!(&self.0.data.#ident))
                    } else {
                        Some(parse_quote!(&self.0.0.#ident))
                    }
                }
            },
        )
    }

    fn generate_combined_builder_struct(&self) -> Item {
        let struct_name = self.struct_builder();
        let cfg_type = self.struct_ident_cfg_builder();
        let status_type = self.struct_ident_builder_status();
        parse_quote! {
            #[derive(Debug, Clone, PartialEq, Default)]
            pub struct #struct_name{
                pub cfg: #cfg_type,
                pub status: #status_type,
            }
        }
    }
    fn generate_combined_deserialize_builder(&self) -> Item {
        let builder_name = self.struct_builder();
        let struct_name = self.struct_type();
        parse_quote! {
            impl resource::DeserializeRosBuilder<#struct_name> for #builder_name {
                type Context=();
                fn init(_ctx: &Self::Context)->Self{
                    Self::default()
                }
                fn append_field(&mut self, key: &[u8], value: Option<&[u8]>) -> resource::AppendFieldResult {
                    match (self.cfg.append_field(key, value),self.status.append_field(key, value)) {
                       (resource::AppendFieldResult::InvalidValue(v),_)|(_,resource::AppendFieldResult::InvalidValue(v))=>resource::AppendFieldResult::InvalidValue(v),
                       (resource::AppendFieldResult::Appended,_)|(_,resource::AppendFieldResult::Appended)=>resource::AppendFieldResult::Appended,
                        _ => resource::AppendFieldResult::UnknownField,
                    }
                }
                fn build(self)->Result<#struct_name,&'static [u8]>{
                    Ok(#struct_name{cfg: self.cfg.build()?, status:self.status.build()?,})
                }
            }
        }
    }

    fn create_id_enum_entry(&self, id_field: &Field) -> RosTypeEntry {
        RosTypeEntry {
            type_name: self.id_struct_ident(id_field),
            field_name: self.id_struct_ident_field(id_field),
            builder: self.id_struct_builder_ident(id_field),
            data: self.id_struct_type(id_field),
            can_update: true,
            can_add: false,
            is_single: false,
        }
    }

    fn create_id_enum_entry_combined(&self, id_field: &Field) -> RosTypeEntry {
        RosTypeEntry {
            type_name: self.id_struct_ident_combined(id_field),
            field_name: self.id_struct_ident_combined_field(id_field),
            builder: self.builder_tuples_for_id_combined(id_field),
            data: self.data_tuples_for_id_combined(id_field),
            can_update: false,
            can_add: false,
            is_single: false,
        }
    }

    fn id_struct_ident_combined(&self, id_field: &Field) -> Ident {
        let struct_name = self.struct_name();
        crate::name2ident(&format!("{struct_name}By_{}WithState", id_field.name))
    }
    fn id_struct_ident_combined_field(&self, id_field: &Field) -> Ident {
        let struct_name = self.struct_name();
        name2field_ident(&format!("{struct_name}By_{}WithState", id_field.name))
    }

    fn generate_has_reference_for_cfg_struct(&self) -> Item {
        Self::generate_has_reference(
            self.struct_type_cfg(),
            self.update_reference_fn(|f| !f.is_read_only),
        )
    }

    fn create_combined_single_impl(&self) -> Item {
        let struct_ident = self.struct_type();
        parse_quote! {
            impl resource::SingleResource for #struct_ident {}
        }
    }

    fn generate_default_for_cfg(&self) -> Item {
        let cfg_name = self.struct_ident_cfg();
        parse_quote! {
            impl Default for #cfg_name {

            }
        }
    }
}

fn name2type(name: &str) -> Type {
    ident2type(crate::name2ident(name))
}

fn ident2type(ident: Ident) -> Type {
    parse_quote!(#ident)
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct Field {
    pub name: Box<str>,
    pub field_type: Option<Box<str>>,
    pub inline_enum: Option<Box<[Box<str>]>>,
    pub is_key: bool,
    pub has_auto: bool,
    pub is_set: bool,
    pub is_range_dot: bool,
    pub is_range_dash: bool,
    pub is_optional: bool,
    pub is_read_only: bool,
    pub is_multiple: bool,
    pub is_hex: bool,
    pub reference: Reference,
    pub has_none: bool,
    pub has_unlimited: bool,
    pub has_disabled: bool,
    pub is_rxtx_pair: bool,
    pub keep_if_none: bool,
    pub default: Option<Box<str>>,
}

impl Field {
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
                                Some(value.split(',').map(|s| s.trim().into()).collect())
                        }

                        "ref" => {
                            if let Some(name) = value.strip_prefix(">") {
                                field.reference = Reference::RefereesTo(name.trim().into());
                            } else {
                                field.reference = Reference::IsReference(value.into());
                            }
                        }
                        "default" => field.default = Some(value.trim().into()),
                        _ => panic!("Invalid field definition: {definition}"),
                    }
                } else {
                    match comp {
                        "id" => field.is_key = true,
                        "ro" => field.is_read_only = true,
                        "auto" => field.has_auto = true,
                        "mu" => field.is_multiple = true,
                        "range" => field.is_range_dash = true,
                        "range-dot" => field.is_range_dot = true,
                        "o" => field.is_optional = true,
                        "hex" => field.is_hex = true,
                        "none" => field.has_none = true,
                        "unlimited" => field.has_unlimited = true,
                        "rxtxpair" => field.is_rxtx_pair = true,
                        "disabled" => field.has_disabled = true,
                        "k" => field.keep_if_none = true,
                        name => {
                            field.field_type = Some(name.trim())
                                .filter(|t| !t.is_empty())
                                .map(|t| t.into())
                        }
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

    fn write_field_line<W: std::fmt::Write>(&self, writer: &mut W) -> std::fmt::Result {
        write!(writer, "{}: ", self.name)?;
        if self.is_key {
            write!(writer, "id; ")?;
        }
        if self.is_read_only {
            write!(writer, "ro; ")?;
        }
        if self.is_multiple {
            write!(writer, "mu; ")?;
        }
        if self.is_range_dash {
            write!(writer, "range; ")?;
        }
        if self.is_range_dot {
            write!(writer, "range-dot; ")?;
        }
        if self.is_optional {
            write!(writer, "o; ")?;
        }
        if self.is_hex {
            write!(writer, "hex; ")?;
        }
        if self.has_none {
            write!(writer, "none; ")?;
        }
        if self.has_unlimited {
            write!(writer, "unlimited; ")?;
        }
        if self.is_rxtx_pair {
            write!(writer, "rxtxpair; ")?;
        }
        if self.has_disabled {
            write!(writer, "disabled; ")?;
        }
        if self.keep_if_none {
            write!(writer, "k; ")?;
        }
        if self.has_auto {
            write!(writer, "auto; ")?;
        }
        match &self.reference {
            Reference::None => {}
            Reference::IsReference(target) => {
                write!(writer, "ref={}; ", target)?;
            }
            Reference::RefereesTo(target) => {
                write!(writer, "ref=>{}; ", target)?;
            }
        }
        if let Some(enum_values) = &self.inline_enum {
            write!(writer, "enum= ")?;
            for (idx, value) in enum_values.iter().enumerate() {
                if idx > 0 {
                    write!(writer, ", {}", value)?;
                } else {
                    write!(writer, "{}", value)?;
                }
            }
            write!(writer, "; ")?;
        }
        if let Some(field_type) = &self.field_type {
            write!(writer, "{}", field_type)?;
        }
        writer.write_char('\n')?;
        Ok(())
    }

    fn compare_and_set_snippet(&self, sub_field: &Option<Ident>) -> Expr {
        let field_name = self.generate_field_name();
        let attribute_name = self.attribute_name();
        let keep_if_none = self.is_optional && self.keep_if_none;
        let field_ref: ExprField = if let Some(field) = sub_field {
            parse_quote!(before.#field.#field_name)
        } else {
            parse_quote!(before.#field_name)
        };
        let cmp: Expr = if keep_if_none {
            parse_quote!(self.#field_name == #field_ref || self.#field_name.is_none())
        } else {
            parse_quote!(self.#field_name == #field_ref)
        };
        let compare_and_set_snippet = parse_quote! {
           if #cmp {
                None
            } else {
                Some(value::KeyValuePair {
                    key: #attribute_name,
                    value: value::RosValue::encode_ros(&self.#field_name),
                })
            }
        };
        compare_and_set_snippet
    }

    fn generate_struct_field_type(&self, enum_field_type: Option<Type>) -> Type {
        let field_type = self.generate_base_field_type(enum_field_type);
        let field_type = if self.is_range_dash {
            parse_quote!(value::PossibleRangeDash<#field_type>)
        } else {
            field_type
        };
        let field_type = if self.is_range_dot {
            parse_quote!(value::PossibleRangeDot<#field_type>)
        } else {
            field_type
        };
        let field_type = if self.is_multiple {
            parse_quote!(std::collections::BTreeSet<#field_type>)
        } else {
            field_type
        };
        if self.is_optional {
            parse_quote!(Option<#field_type>)
        } else {
            field_type
        }
    }
    fn generate_builder_type(&self, enum_field_type: Option<Type>) -> Type {
        let field_type = self.generate_struct_field_type(enum_field_type);
        if self.is_optional || self.is_multiple {
            field_type
        } else {
            parse_quote!(Option<#field_type>)
        }
        /*let field_type = self.generate_base_field_type(enum_field_type);
        let field_type = if self.is_range {
            parse_quote!(value::PossibleRange<#field_type>)
        } else {
            field_type
        };
        if self.is_multiple {
            parse_quote!(std::collections::BTreeSet<#field_type>)
        } else {
            parse_quote!(Option<#field_type>)
        }*/
    }

    fn generate_base_field_type(&self, enum_field_type: Option<Type>) -> Type {
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
            .unwrap_or(parse_quote!(ascii::AsciiString));
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
        if self.is_rxtx_pair {
            parse_quote!(value::RxTxPair<#field_type>)
        } else {
            field_type
        }
    }

    fn attribute_name(&self) -> Literal {
        Literal::byte_string(self.name.as_bytes())
    }

    fn generate_field_name(&self) -> Ident {
        let field_name = cleanup_field_name(self.name.as_ref()).to_case(Case::Snake);

        let field_name = if KEYWORDS.contains(field_name.as_str())
            || field_name
                .chars()
                .next()
                .map(|ch| ch.is_numeric())
                .unwrap_or(true)
        {
            format!("_{field_name}")
        } else {
            field_name
        };
        Ident::new(&field_name, Span::mixed_site())
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

fn parse_path(name: &str) -> Box<[Box<str>]> {
    let path: Box<[Box<str>]> = name
        .trim()
        .split('/')
        .map(|s| s.to_string().into_boxed_str())
        .collect();
    path
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_entity_parser() {
        let data = include_str!("../../ros_model/system.txt");
        let lines = data.lines();
        let collected_entities = Entity::parse_lines(lines);
        println!("{:#?}", collected_entities);
    }
}
