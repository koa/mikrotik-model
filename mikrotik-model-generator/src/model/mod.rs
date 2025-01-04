use crate::{cleanup_field_name, derive_ident, generate_enums, KEYWORDS};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, Literal, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use syn::{
    parse_quote,
    punctuated::Punctuated,
    token::{Colon, Comma},
    Expr, ExprArray, ExprMatch, ExprStruct, FieldValue, FieldsNamed, FnArg, ImplItem,
    Item, ItemImpl, Member, Path, PathSegment, Token, Type, TypePath,
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
    pub can_add: bool,
}

pub struct RosTypeEntry {
    pub name: Ident,
    pub builder: Type,
    pub data: Type,
}

impl Entity {
    pub fn generate_code(
        &self,
    ) -> (
        impl IntoIterator<Item=Item>,
        impl IntoIterator<Item=RosTypeEntry>,
    ) {
        let mut inline_enums = HashMap::new();
        let (struct_field_types, builder_field_types) = self.create_field_types(&mut inline_enums);
        let mut items: Vec<Item> = Vec::with_capacity(5);
        let mut enum_entries: Vec<RosTypeEntry> = Vec::with_capacity(1);

        let has_cfg_struct = self
            .modifiable_fields_iterator(&struct_field_types)
            .next()
            .is_some();
        let has_readonly_fields = self
            .read_only_fields_iterator(&struct_field_types)
            .next()
            .is_some();

        if has_cfg_struct {
            items.push(self.create_cfg_struct(&struct_field_types));
            items.push(self.create_cfg_builder_struct(&builder_field_types));
            enum_entries.push(self.create_cfg_enum_entry());
            items.push(self.create_deserialize_for_cfg_struct());
            items.push(self.generate_ros_resource_for_cfg());
            items.push(self.create_deserialize_builder_for_cfg());
            items.push(self.create_cfg_resource(&struct_field_types));
            let mut cfg_struct_items = Vec::new();
            if self.is_single {
                items.push(self.single_resource_cfg());
                items.push(self.updateable_cfg());
            } else {
                if self.can_add {
                    cfg_struct_items.push(self.create_constructor(&struct_field_types));
                    items.push(self.create_createable_impl(&struct_field_types));
                }
                for (id_field, (id_field_type, id_builder_type)) in self
                    .fields
                    .iter()
                    .zip(struct_field_types.iter().zip(builder_field_types.iter()))
                    .filter(|(f, _)| f.is_key)
                {
                    items.push(self.generate_deserialize_for_id(id_field));
                    items.push(self.generate_ros_resource_for_id(id_field));
                    enum_entries.push(self.create_id_enum_entry(id_field));
                    if id_field.is_read_only {
                        items.push(self.generate_id_struct_external(id_field, id_field_type));
                        items.push(self.generate_id_builder_external(id_field, id_builder_type));
                        items.push(self.generate_deserialize_builder_for_id_external(id_field));
                        items.push(self.generate_keyed_for_id_external(id_field, id_field_type));
                        items.push(self.generate_cfg_for_id_external(id_field));
                    } else {
                        items.push(self.generate_id_struct_internal(id_field));
                        items.push(self.generate_id_builder_internal(id_field));
                        items.push(self.generate_deserialize_builder_for_id_internal(id_field));
                        items.push(self.generate_keyed_for_id_internal(id_field, id_field_type));
                        items.push(self.generate_cfg_for_id_internal(id_field));
                    }
                    if has_readonly_fields {
                        items.push(self.generate_deserialize_for_id_combined(id_field));
                        items.push(self.generate_ros_for_id_combined(id_field));
                        items.push(self.generate_deserialize_builder_for_id_combined(id_field));
                        items.push(self.generate_keyed_for_id_combined(id_field, id_field_type));
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
            items.push(self.create_status_struct(&struct_field_types));
            items.push(self.create_status_builder_struct(&builder_field_types));
            items.push(self.create_deserialize_for_status_struct());
            items.push(self.create_ros_resource_for_status());
            items.push(self.create_deserialize_builder_for_status());
            enum_entries.push(self.create_status_enum_entry());
        }
        if has_cfg_struct && has_readonly_fields {
            enum_entries.push(self.create_enum_entry());
            items.push(self.generate_combined_struct());
            items.push(self.generate_combined_builder_struct());
            items.push(self.generate_deserialize_for_combined_struct());
            items.push(self.generate_combined_deserialize_builder());
            items.push(self.generate_ros_for_combined_struct());
        }
        (
            generate_enums(&inline_enums)
                .chain(items)
                .collect::<Vec<_>>(),
            enum_entries,
        )
    }

    fn create_cfg_builder_struct(&self, builder_field_types: &[Type]) -> Item {
        let struct_name = self.struct_ident_cfg_builder();
        let fields = self.modifiable_field_declarations(builder_field_types);
        parse_quote! {
            #[derive(Debug, Clone, PartialEq, Default)]
            pub struct #struct_name #fields
        }
    }

    fn create_cfg_struct(&self, struct_field_types: &[Type]) -> Item {
        let struct_name = self.struct_type_cfg();
        let fields = self.modifiable_field_declarations(struct_field_types);
        parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #struct_name #fields
        }
    }

    fn generate_ros_for_combined_struct(&self) -> Item {
        let struct_ident = self.struct_type();
        let path = self.generate_path();
        parse_quote! {
            impl resource::RosResource for #struct_ident {
                fn path()->&'static [u8]{
                    #path
                }
            }
        }
    }

    fn generate_deserialize_for_combined_struct(&self) -> Item {
        Self::generate_deserialize(
            self.struct_type(),
            self.struct_builder(),
            self.struct_ident(),
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

    fn generate_keyed_for_id_combined(&self, id_field: &Field, id_field_type: &Type) -> Item {
        let field_name = id_field.attribute_name();
        let struct_ident_status = self.struct_status_type();
        let id_struct_ident = self.id_struct_type(id_field);
        let item = parse_quote! {
            impl resource::KeyedResource for (#id_struct_ident, #struct_ident_status) {
                type Key = #id_field_type;

                fn key_name() -> &'static [u8] {
                    #field_name
                }

                fn key_value(&self) -> &#id_field_type {
                    self.0.key_value()
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
        )
    }

    fn generate_deserialize(data_type: Type, builder_type: Type, variant_name: Ident) -> Item {
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
        let item = parse_quote! {
            impl resource::CfgResource for #id_struct_ident {
                #[allow(clippy::needless_lifetimes)]
                fn changed_values<'a, 'b>(
                    &'a self,
                    before: &'b Self,
                ) -> impl Iterator<Item = value::KeyValuePair<'a>> {
                    self.0.changed_values(&before.0)
                }
            }
        };
        item
    }

    fn generate_keyed_for_id_internal(&self, id_field: &Field, id_field_type: &Type) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        let field_name = id_field.attribute_name();
        let id_field_name = id_field.generate_field_name();

        let item = parse_quote! {
            impl resource::KeyedResource for #id_struct_ident {
                type Key = #id_field_type;

                fn key_name() -> &'static [u8] {
                    #field_name
                }

                fn key_value(&self) -> &#id_field_type {
                    &self.0.#id_field_name
                }
            }
        };
        item
    }

    fn generate_id_struct_internal(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        let struct_ident_cfg = self.struct_type_cfg();

        let item = parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #id_struct_ident(pub #struct_ident_cfg);
        };
        item
    }

    fn generate_cfg_for_id_external(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);

        let item = parse_quote! {
            impl resource::CfgResource for #id_struct_ident {
                #[allow(clippy::needless_lifetimes)]
                fn changed_values<'a, 'b>(
                    &'a self,
                    before: &'b Self,
                ) -> impl Iterator<Item = value::KeyValuePair<'a>> {
                    self.data.changed_values(&before.data)
                }
            }
        };
        item
    }

    fn generate_keyed_for_id_external(&self, id_field: &Field, id_field_type: &Type) -> Item {
        let field_name = id_field.attribute_name();
        let id_field_name = id_field.generate_field_name();
        let id_struct_ident = self.id_struct_type(id_field);

        parse_quote! {
            impl resource::KeyedResource for #id_struct_ident {
                type Key = #id_field_type;

                fn key_name() -> &'static [u8] {
                    #field_name
                }

                fn key_value(&self) -> &#id_field_type {
                    &self.#id_field_name
                }
            }
        }
    }

    fn generate_deserialize_for_id(&self, id_field: &Field) -> Item {
        let id_struct_ident = self.id_struct_type(id_field);
        let builder_name = self.id_struct_builder_ident(id_field);
        Self::generate_deserialize(
            id_struct_ident,
            builder_name,
            self.id_struct_ident(id_field),
        )
    }

    fn generate_id_struct_external(&self, id_field: &Field, id_field_type: &Type) -> Item {
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
        name2ident(&format!("{struct_name}By_{}", id_field.name))
    }

    fn id_struct_builder_ident(&self, id_field: &Field) -> Type {
        let struct_name = self.struct_name();
        name2type(
            cleanup_field_name(&format!("{struct_name}By_{}_Builder", id_field.name))
                .to_case(Case::UpperCamel)
                .as_str(),
        )
    }

    fn create_deserialize_for_status_struct(&self) -> Item {
        Self::generate_deserialize(
            self.struct_status_type(),
            self.struct_ident_builder_status(),
            self.struct_status_ident(),
        )
    }

    fn create_status_struct(&self, field_types: &[Type]) -> Item {
        let fields_named_status = self.create_status_fields(field_types);
        let struct_ident_status = self.struct_status_type();
        parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #struct_ident_status #fields_named_status
        }
    }

    fn create_createable_impl(&self, field_types: &[Type]) -> Item {
        let struct_ident_cfg = self.struct_type_cfg();
        let create_values_array = self.modifiable_field_creators(field_types);
        let item = parse_quote! {
            impl resource::Creatable for #struct_ident_cfg{
                fn calculate_create(&self) -> resource::ResourceMutation<'_> {
                    resource::ResourceMutation {
                        resource: <#struct_ident_cfg as resource::RosResource>::path(),
                        operation: resource::ResourceMutationOperation::Add,
                        fields: Box::new(#create_values_array),
                    }
                }
            }
        };
        item
    }

    fn create_constructor(&self, field_types: &[Type]) -> ImplItem {
        let mut params: Punctuated<FnArg, Comma> = Default::default();
        let mut fields_named: Punctuated<FieldValue, Token![,]> = Punctuated::new();

        for (field, field_type) in self.modifiable_fields_iterator(field_types) {
            let field_name = field.generate_field_name();
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
        let item = parse_quote! {
            impl resource::Updatable for #struct_ident_cfg {
                fn calculate_update<'a>(&'a self, from: &'a Self) -> resource::ResourceMutation<'a> {
                    resource::ResourceMutation {
                        resource: <#struct_ident_cfg as resource::RosResource>::path(),
                        operation: resource::ResourceMutationOperation::UpdateSingle,
                        fields: resource::CfgResource::changed_values(self,from).collect(),
                    }
                }
            }
        };
        item
    }

    fn single_resource_cfg(&self) -> Item {
        let struct_ident_cfg = self.struct_type_cfg();
        let item = parse_quote! {
            impl resource::SingleResource for #struct_ident_cfg {}
        };
        item
    }

    fn create_cfg_resource(&self, field_types: &[Type]) -> Item {
        let struct_ident_cfg = self.struct_type_cfg();
        let changed_values_array = self.modifiable_field_updaters(field_types);
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
        Self::generate_ros_resource(self.struct_type_cfg(), self.generate_path())
    }

    fn generate_ros_resource(struct_ident_cfg: Type, path: Literal) -> Item {
        parse_quote! {
            impl resource::RosResource for #struct_ident_cfg {
                fn path()->&'static [u8]{
                    #path
                }
            }
        }
    }

    fn generate_path(&self) -> Literal {
        Literal::byte_string(self.path.join("/").as_bytes())
    }

    fn create_deserialize_for_cfg_struct(&self) -> Item {
        Self::generate_deserialize(
            self.struct_type_cfg(),
            self.struct_ident_cfg_builder(),
            self.struct_ident_cfg(),
        )
    }

    fn create_cfg_enum_entry(&self) -> RosTypeEntry {
        let struct_ident = self.struct_ident_cfg();
        let builder = self.struct_ident_cfg_builder();
        let data = self.struct_type_cfg();
        RosTypeEntry {
            name: struct_ident,
            builder,
            data,
        }
    }
    fn create_status_enum_entry(&self) -> RosTypeEntry {
        let struct_ident = self.struct_status_ident();
        let builder = self.struct_ident_builder_status();
        let data = self.struct_status_type();
        RosTypeEntry {
            name: struct_ident,
            builder,
            data,
        }
    }
    fn create_enum_entry(&self) -> RosTypeEntry {
        let struct_ident = self.struct_ident();
        let builder = self.struct_builder();
        let data = self.struct_type();
        RosTypeEntry {
            name: struct_ident,
            builder,
            data,
        }
    }

    fn modifiable_field_creators(&self, field_types: &[Type]) -> ExprArray {
        let mut create_values_array = ExprArray {
            attrs: vec![],
            bracket_token: Default::default(),
            elems: Default::default(),
        };
        for (field, field_type) in self.modifiable_fields_iterator(field_types) {
            let field_key = Literal::byte_string(field.name.as_bytes());
            let field_name = field.generate_field_name();
            create_values_array.elems.push(parse_quote! {
                value::KeyValuePair {
                    key: #field_key,
                    value: <#field_type as value::RosValue>::encode_ros(&self.#field_name),
                }
            });
        }
        create_values_array
    }

    fn modifiable_field_updaters(&self, field_types: &[Type]) -> ExprArray {
        let mut changed_values_array = ExprArray {
            attrs: vec![],
            bracket_token: Default::default(),
            elems: Default::default(),
        };
        for (field, _) in self.modifiable_fields_iterator(field_types) {
            changed_values_array
                .elems
                .push(field.compare_and_set_snippet());
        }
        changed_values_array
    }

    fn modifiable_field_declarations(&self, field_types: &[Type]) -> FieldsNamed {
        let mut fields_named_cfg = FieldsNamed {
            brace_token: Default::default(),
            named: Default::default(),
        };
        for (f, field_type) in self.modifiable_fields_iterator(field_types) {
            let field_name = f.generate_field_name();
            fields_named_cfg
                .named
                .push(parse_quote!(pub #field_name: #field_type));
        }
        fields_named_cfg
    }

    fn modifiable_fields_iterator<'a>(
        &'a self,
        field_types: &'a [Type],
    ) -> impl Iterator<Item=(&'a Field, &'a Type)> {
        self.fields
            .iter()
            .zip(field_types.iter())
            .filter(|(f, _)| !f.is_read_only)
    }

    fn create_status_fields(&self, field_types: &[Type]) -> FieldsNamed {
        let mut fields_named_status = FieldsNamed {
            brace_token: Default::default(),
            named: Default::default(),
        };
        for (field, field_type) in self.read_only_fields_iterator(field_types) {
            let field_name = field.generate_field_name();
            let field_def = parse_quote!(
                pub #field_name: #field_type
            );
            fields_named_status.named.push(field_def);
        }
        fields_named_status
    }

    fn read_only_fields_iterator<'a>(
        &'a self,
        field_types: &'a [Type],
    ) -> impl Iterator<Item=(&'a Field, &'a Type)> {
        self.fields
            .iter()
            .zip(field_types.iter())
            .filter(|(f, _)| f.is_read_only)
    }

    fn create_field_types(
        &self,
        inline_enums: &mut HashMap<Box<str>, Box<[Box<str>]>>,
    ) -> (Vec<Type>, Vec<Type>) {
        let mut struct_field_types = Vec::new();
        let mut builder_field_types = Vec::new();
        for field in self.fields.iter() {
            let enum_type = if let Some(enum_values) = field.inline_enum.as_ref() {
                let struct_name = self.struct_name();
                let enum_name = format!("{struct_name}_{}", field.name);
                let enum_type = Ident::new(&derive_ident(&enum_name), Span::call_site());
                inline_enums.insert(enum_name.into_boxed_str(), enum_values.clone());
                Some(parse_quote!(#enum_type))
            } else {
                None
            };
            struct_field_types.push(field.generate_struct_field_type(enum_type.clone()));
            builder_field_types.push(field.generate_builder_type(enum_type));
        }
        (struct_field_types, builder_field_types)
    }

    fn struct_type_cfg(&self) -> Type {
        ident2type(self.struct_ident_cfg())
    }

    fn struct_ident_cfg(&self) -> Ident {
        let struct_name = self.struct_name();
        name2ident(&format!("{struct_name}Cfg"))
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
        let string = format!("{struct_name}State");
        name2ident(&string)
    }

    fn struct_ident_builder_status(&self) -> Type {
        let struct_name = self.struct_name();
        name2type(&format!("{struct_name}StateBuilder"))
    }

    fn struct_type(&self) -> Type {
        ident2type(self.struct_ident())
    }

    fn struct_ident(&self) -> Ident {
        name2ident(&self.struct_name())
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

    fn generate_deserialize_for_builder<'a, F: Fn() -> I, I: Iterator<Item=&'a Field>>(
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

    fn generate_id_builder_external(&self, id_field: &Field, id_builder_type: &Type) -> Item {
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

    fn create_status_builder_struct(&self, builder_field_types: &[Type]) -> Item {
        let struct_name = self.struct_ident_builder_status();
        let fields = self.create_status_fields(builder_field_types);
        parse_quote! {
            #[derive(Debug, Clone, PartialEq, Default)]
            pub struct #struct_name #fields
        }
    }

    fn create_ros_resource_for_status(&self) -> Item {
        let struct_ident_status = self.struct_status_type();
        let path = self.generate_path();
        parse_quote! {
            impl resource::RosResource for #struct_ident_status {
                fn path()->&'static [u8]{
                    #path
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
        Self::generate_ros_resource(self.id_struct_type(id_field), self.generate_path())
    }

    fn generate_ros_for_id_combined(&self, id_field: &Field) -> Item {
        let data_type = self.data_tuples_for_id_combined(id_field);
        Self::generate_ros_resource(data_type, self.generate_path())
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
            name: self.id_struct_ident(id_field),
            builder: self.id_struct_builder_ident(id_field),
            data: self.id_struct_type(id_field),
        }
    }

    fn create_id_enum_entry_combined(&self, id_field: &Field) -> RosTypeEntry {
        RosTypeEntry {
            name: self.id_struct_ident_combined(id_field),
            builder: self.builder_tuples_for_id_combined(id_field),
            data: self.data_tuples_for_id_combined(id_field),
        }
    }

    fn id_struct_ident_combined(&self, id_field: &Field) -> Ident {
        let struct_name = self.struct_name();
        name2ident(&format!("{struct_name}By_{}WithState", id_field.name))
    }
}

fn name2type(name: &str) -> Type {
    ident2type(name2ident(name))
}

fn ident2type(ident: Ident) -> Type {
    parse_quote!(#ident)
}

fn name2ident(name: &str) -> Ident {
    Ident::new(
        cleanup_field_name(name).to_case(Case::UpperCamel).as_str(),
        Span::call_site(),
    )
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
    fn compare_and_set_snippet(&self) -> Expr {
        let field_name = self.generate_field_name();

        let attribute_name = self.attribute_name();
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
        compare_and_set_snippet
    }

    fn generate_struct_field_type(&self, enum_field_type: Option<Type>) -> Type {
        self.generate_field_type(enum_field_type, false)
    }
    fn generate_builder_type(&self, enum_field_type: Option<Type>) -> Type {
        self.generate_field_type(enum_field_type, true)
    }
    fn generate_field_type(&self, enum_field_type: Option<Type>, for_builder: bool) -> Type {
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
        let field_type = if self.is_rxtx_pair {
            parse_quote!(value::RxTxPair<#field_type>)
        } else {
            field_type
        };
        if self.is_multiple {
            parse_quote!(std::collections::HashSet<#field_type>)
        } else if self.is_optional || for_builder {
            parse_quote!(Option<#field_type>)
        } else {
            field_type
        }
    }

    fn attribute_name(&self) -> Literal {
        Literal::byte_string(self.name.as_bytes())
    }

    fn generate_field_name(&self) -> Ident {
        let field_name = cleanup_field_name(self.name.as_ref()).to_case(Case::Snake);
        let field_name = if KEYWORDS.contains(field_name.as_str()) {
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
                can_add: false,
            }) {
                collected_entities.push(entity);
            }
        } else if let Some(name) = line.strip_prefix("1/") {
            let path = parse_path(name);
            if let Some(entity) = current_entity.replace(Entity {
                path,
                fields: vec![],
                is_single: true,
                can_add: false,
            }) {
                collected_entities.push(entity);
            }
        } else if let Some(name) = line.strip_prefix("*/") {
            let path = parse_path(name);
            if let Some(entity) = current_entity.replace(Entity {
                path,
                fields: vec![],
                is_single: false,
                can_add: true,
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
