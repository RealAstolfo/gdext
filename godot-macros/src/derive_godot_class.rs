/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::{bail, ensure_kv_empty, ident, parse_kv_group, path_is_single, KvMap, KvValue};
use crate::{util, ParseResult};
use proc_macro2::{Ident, Punct, Span, TokenStream};
use quote::spanned::Spanned;
use quote::{format_ident, quote};
use venial::{Attribute, NamedField, Struct, StructFields, TyExpr};

pub fn transform(input: TokenStream) -> ParseResult<TokenStream> {
    let decl = venial::parse_declaration(input)?;

    let class = decl
        .as_struct()
        .ok_or(venial::Error::new("Not a valid struct"))?;

    let struct_cfg = parse_struct_attributes(class)?;
    let fields = parse_fields(class)?;

    let base_ty = &struct_cfg.base_ty;
    let base_ty_str = struct_cfg.base_ty.to_string();
    let class_name = &class.name;
    let class_name_str = class.name.to_string();
    let inherits_macro = format_ident!("inherits_transitive_{}", &base_ty_str);

    let prv = quote! { ::godot::private };
    let deref_impl = make_deref_impl(class_name, &fields);

    let godot_exports_impl = make_exports_impl(class_name, &fields);

    let (godot_init_impl, create_fn);
    if struct_cfg.has_generated_init {
        godot_init_impl = make_godot_init_impl(class_name, fields);
        create_fn = quote! { Some(#prv::callbacks::create::<#class_name>) };
    } else {
        godot_init_impl = TokenStream::new();
        create_fn = quote! { None };
    };

    Ok(quote! {
        impl ::godot::obj::GodotClass for #class_name {
            type Base = ::godot::engine::#base_ty;
            type Declarer = ::godot::obj::dom::UserDomain;
            type Mem = <Self::Base as ::godot::obj::GodotClass>::Mem;

            const CLASS_NAME: &'static str = #class_name_str;
        }

        #godot_init_impl
        #godot_exports_impl
        #deref_impl

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_str,
            component: #prv::PluginComponent::ClassDef {
                base_class_name: #base_ty_str,
                generated_create_fn: #create_fn,
                free_fn: #prv::callbacks::free::<#class_name>,
            },
        });

        #prv::class_macros::#inherits_macro!(#class_name);
    })
}

/// Returns the name of the base and the default mode
fn parse_struct_attributes(class: &Struct) -> ParseResult<ClassAttributes> {
    let mut base = ident("RefCounted");
    //let mut new_mode = GodotConstructMode::AutoGenerated;
    let mut has_generated_init = false;

    // #[func] attribute on struct
    if let Some((span, mut map)) = parse_class_attr(&class.attributes)? {
        //println!(">>> CLASS {class}, MAP: {map:?}", class = class.name);

        if let Some(kv_value) = map.remove("base") {
            if let KvValue::Ident(override_base) = kv_value {
                base = override_base;
            } else {
                bail("Invalid value for 'base' argument", span)?;
            }
        }

        /*if let Some(kv_value) = map.remove("new") {
            match kv_value {
                KvValue::Ident(ident) if ident == "fn" => new_mode = GodotConstructMode::FnNew,
                KvValue::Ident(ident) if ident == "none" => new_mode = GodotConstructMode::None,
                _ => bail(
                    "Invalid value for 'new' argument; must be 'fn' or 'none'",
                    span,
                )?,
            }
        }*/
        if let Some(kv_value) = map.remove("init") {
            match kv_value {
                KvValue::None => has_generated_init = true,
                _ => bail("Argument 'init' must not have a value", span)?,
            }
        }
        ensure_kv_empty(map, span)?;
    }

    Ok(ClassAttributes {
        base_ty: base,
        has_generated_init,
    })
}

/// Returns field names and 1 base field, if available
fn parse_fields(class: &Struct) -> ParseResult<Fields> {
    let mut all_field_names = vec![];
    let mut exported_fields = vec![];
    let mut base_field = Option::<Field>::None;

    let fields: Vec<(NamedField, Punct)> = match &class.fields {
        StructFields::Unit => {
            vec![]
        }
        StructFields::Tuple(_) => bail(
            "#[derive(GodotClass)] not supported for tuple structs",
            &class.fields,
        )?,
        StructFields::Named(fields) => fields.fields.inner.clone(),
    };

    // Attributes on struct fields
    for (field, _punct) in fields {
        let mut is_base = false;

        // #[base] or #[export]
        for attr in field.attributes.iter() {
            if let Some(path) = attr.get_single_path_segment() {
                if path == "base" {
                    is_base = true;
                    if let Some(prev_base) = base_field {
                        bail(
                            &format!(
                                "#[base] allowed for at most 1 field, already applied to '{}'",
                                prev_base.name
                            ),
                            attr,
                        )?;
                    }
                    base_field = Some(Field::new(&field))
                } else if path == "export" {
                    match parse_kv_group(&attr.value) {
                        Ok(export_kv) => {
                            let exported_field =
                                ExportedField::new_from_kv(Field::new(&field), attr, export_kv)?;
                            exported_fields.push(exported_field);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                }
            }
        }

        // Exported or Rust-only fields
        if !is_base {
            all_field_names.push(field.name.clone())
        }
    }

    Ok(Fields {
        all_field_names,
        base_field,
        exported_fields,
    })
}

/// Parses a `#[class(...)]` attribute
fn parse_class_attr(attributes: &Vec<Attribute>) -> ParseResult<Option<(Span, KvMap)>> {
    let mut godot_attr = None;
    for attr in attributes.iter() {
        let path = &attr.path;
        if path_is_single(path, "class") {
            if godot_attr.is_some() {
                bail(
                    "Only one #[class] attribute per item (struct, fn, ...) allowed",
                    attr,
                )?;
            }

            let map = util::parse_kv_group(&attr.value)?;
            godot_attr = Some((attr.__span(), map));
        }
    }
    Ok(godot_attr)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// General helpers

struct ClassAttributes {
    base_ty: Ident,
    has_generated_init: bool,
}

struct Fields {
    all_field_names: Vec<Ident>,
    base_field: Option<Field>,
    exported_fields: Vec<ExportedField>,
}

struct Field {
    name: Ident,
    _ty: TyExpr,
}

impl Field {
    fn new(field: &NamedField) -> Self {
        Self {
            name: field.name.clone(),
            _ty: field.ty.clone(),
        }
    }
}

struct ExportedField {
    field: Field,
    getter: String,
    setter: String,
    variant_type: String,
}

impl ExportedField {
    pub fn new_from_kv(
        field: Field,
        attr: &Attribute,
        mut map: KvMap,
    ) -> ParseResult<ExportedField> {
        let getter = Self::require_key_value(&mut map, "getter", attr)?;
        let setter = Self::require_key_value(&mut map, "setter", attr)?;
        let variant_type = Self::require_key_value(&mut map, "variant_type", attr)?;

        ensure_kv_empty(map, attr.__span())?;

        Ok(ExportedField {
            field,
            getter,
            setter,
            variant_type,
        })
    }

    fn require_key_value(map: &mut KvMap, key: &str, attr: &Attribute) -> ParseResult<String> {
        if let Some(value) = map.remove(key) {
            if let KvValue::Lit(value) = value {
                Ok(value)
            } else {
                bail(
                    format!(
                        "#[export] attribute {} with a non-literal variant_type",
                        key
                    ),
                    attr,
                )?
            }
        } else {
            bail(format!("#[export] attribute without a {}", key), attr)
        }
    }
}

fn make_godot_init_impl(class_name: &Ident, fields: Fields) -> TokenStream {
    let base_init = if let Some(Field { name, .. }) = fields.base_field {
        quote! { #name: base, }
    } else {
        TokenStream::new()
    };

    let rest_init = fields.all_field_names.into_iter().map(|field| {
        quote! { #field: std::default::Default::default(), }
    });

    quote! {
        impl ::godot::obj::cap::GodotInit for #class_name {
            fn __godot_init(base: ::godot::obj::Base<Self::Base>) -> Self {
                Self {
                    #( #rest_init )*
                    #base_init
                }
            }
        }
    }
}

fn make_deref_impl(class_name: &Ident, fields: &Fields) -> TokenStream {
    let base_field = if let Some(Field { name, .. }) = &fields.base_field {
        name
    } else {
        return TokenStream::new();
    };

    quote! {
        impl std::ops::Deref for #class_name {
            type Target = <Self as ::godot::obj::GodotClass>::Base;

            fn deref(&self) -> &Self::Target {
                &*self.#base_field
            }
        }
        impl std::ops::DerefMut for #class_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut *self.#base_field
            }
        }
    }
}

fn make_exports_impl(class_name: &Ident, fields: &Fields) -> TokenStream {
    let export_tokens = fields
        .exported_fields
        .iter()
        .map(|exported_field: &ExportedField| {
            use std::str::FromStr;
            let name = exported_field.field.name.to_string();
            let getter = proc_macro2::Literal::from_str(&exported_field.getter).unwrap();
            let setter = proc_macro2::Literal::from_str(&exported_field.setter).unwrap();
            let vtype = &exported_field.variant_type;
            let variant_type: TokenStream = vtype[1..vtype.len() - 1].parse().unwrap();
            quote! {
                let class_name = ::godot::builtin::StringName::from(#class_name::CLASS_NAME);
                let property_info = ::godot::builtin::meta::PropertyInfo::new(
                    #variant_type,
                    ::godot::builtin::meta::ClassName::new::<#class_name>(),
                    ::godot::builtin::StringName::from(#name),
                );
                let property_info_sys = property_info.property_sys();

                let getter_string_name = ::godot::builtin::StringName::from(#getter);
                let setter_string_name = ::godot::builtin::StringName::from(#setter);
                unsafe {
                    ::godot::sys::interface_fn!(classdb_register_extension_class_property)(
                        ::godot::sys::get_library(),
                        class_name.string_sys(),
                        std::ptr::addr_of!(property_info_sys),
                        setter_string_name.string_sys(),
                        getter_string_name.string_sys(),
                    );
                }
            }
        });
    quote! {
        impl ::godot::obj::cap::ImplementsGodotExports for #class_name {
            fn __register_exports() {
                #(
                    {
                        #export_tokens
                    }
                )*
            }
        }
    }
}
