//! Derive macros for the `llsd-rs` crate.
//! Re-exported automatically when enabling the `derive` feature on `llsd-rs`.
#![allow(clippy::derivable_impls)]
#![allow(clippy::enum_variant_names)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, Data, DeriveInput, Fields, Ident, Lit, Type, parse_macro_input};

// Container / field attribute models -----------------------------------------------------------
#[derive(Debug, Clone, Default)]
struct ContainerAttributes {
    rename_all: Option<RenameRule>,
    deny_unknown_fields: bool,
}

#[derive(Debug, Clone)]
struct FieldAttributes {
    rename: Option<String>,
    skip: bool,
    skip_serializing: bool,
    skip_deserializing: bool,
    default: DefaultType,
    flatten: bool,
    with: Option<syn::Path>,
}
impl Default for FieldAttributes {
    fn default() -> Self {
        Self {
            rename: None,
            skip: false,
            skip_serializing: false,
            skip_deserializing: false,
            default: DefaultType::None,
            flatten: false,
            with: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
enum DefaultType {
    #[default]
    None,
    Default,
    Path(syn::Path),
}

#[derive(Debug, Clone, Copy)]
enum RenameRule {
    Snake,
    Kebab,
    Camel,
    Pascal,
    ScreamingSnake,
    Lower,
    Upper,
}
impl RenameRule {
    fn apply(&self, name: &str) -> String {
        match self {
            RenameRule::Snake => to_snake_case(name),
            RenameRule::Kebab => to_snake_case(name).replace('_', "-"),
            RenameRule::Camel => to_camel_case(name),
            RenameRule::Pascal => to_pascal_case(name),
            RenameRule::ScreamingSnake => to_snake_case(name).to_uppercase(),
            RenameRule::Lower => name.to_lowercase(),
            RenameRule::Upper => name.to_uppercase(),
        }
    }
}

// Parsing -------------------------------------------------------------------------------------
fn parse_container_attributes(attrs: &[Attribute]) -> syn::Result<ContainerAttributes> {
    let mut out = ContainerAttributes::default();
    for attr in attrs {
        if !attr.path().is_ident("llsd") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    out.rename_all = Some(match s.value().as_str() {
                        "snake_case" => RenameRule::Snake,
                        "kebab-case" => RenameRule::Kebab,
                        "camelCase" => RenameRule::Camel,
                        "PascalCase" => RenameRule::Pascal,
                        "SCREAMING_SNAKE_CASE" => RenameRule::ScreamingSnake,
                        "lowercase" => RenameRule::Lower,
                        "UPPERCASE" => RenameRule::Upper,
                        _ => return Err(syn::Error::new(s.span(), "Invalid rename_all value")),
                    });
                    Ok(())
                } else {
                    Err(syn::Error::new(lit.span(), "Expected string literal"))
                }
            } else if meta.path.is_ident("deny_unknown_fields") {
                out.deny_unknown_fields = true;
                Ok(())
            } else {
                Err(meta.error("Unknown container attribute"))
            }
        })?;
    }
    Ok(out)
}

fn parse_field_attributes(attrs: &[Attribute]) -> syn::Result<FieldAttributes> {
    let mut out = FieldAttributes::default();
    for attr in attrs {
        if !attr.path().is_ident("llsd") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    out.rename = Some(s.value());
                    Ok(())
                } else {
                    Err(syn::Error::new(lit.span(), "Expected string literal"))
                }
            } else if meta.path.is_ident("skip") {
                out.skip = true;
                Ok(())
            } else if meta.path.is_ident("skip_serializing") {
                out.skip_serializing = true;
                Ok(())
            } else if meta.path.is_ident("skip_deserializing") {
                out.skip_deserializing = true;
                Ok(())
            } else if meta.path.is_ident("default") {
                if meta.input.peek(syn::token::Eq) {
                    let value = meta.value()?;
                    let path: syn::Path = value.parse()?;
                    out.default = DefaultType::Path(path);
                } else {
                    out.default = DefaultType::Default;
                }
                Ok(())
            } else if meta.path.is_ident("flatten") {
                out.flatten = true;
                Ok(())
            } else if meta.path.is_ident("with") {
                let value = meta.value()?;
                let path: syn::Path = value.parse()?;
                out.with = Some(path);
                Ok(())
            } else {
                Err(meta.error("Unknown field attribute"))
            }
        })?;
    }
    Ok(out)
}

// Trait impl generation -----------------------------------------------------------------------
#[proc_macro_derive(LlsdFrom, attributes(llsd))]
pub fn derive_llsd_from(input: TokenStream) -> TokenStream {
    expand(input, Mode::From)
}
#[proc_macro_derive(LlsdInto, attributes(llsd))]
pub fn derive_llsd_into(input: TokenStream) -> TokenStream {
    expand(input, Mode::Into)
}
#[proc_macro_derive(LlsdFromTo, attributes(llsd))]
pub fn derive_llsd_from_to(input: TokenStream) -> TokenStream {
    expand(input, Mode::Both)
}

#[derive(Clone, Copy)]
enum Mode {
    From,
    Into,
    Both,
}

fn expand(input: TokenStream, mode: Mode) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match impl_expand(ast, mode) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

// Internal representation of a parsed field
struct FieldInfo {
    ident: Ident,
    attrs: FieldAttributes,
    llsd_name: String,
    is_option: bool,
}

fn impl_expand(ast: DeriveInput, mode: Mode) -> syn::Result<proc_macro2::TokenStream> {
    let name = &ast.ident;
    let container_attrs = parse_container_attributes(&ast.attrs)?;
    let data = match ast.data {
        Data::Struct(s) => s,
        _ => return Err(syn::Error::new_spanned(name, "Only structs supported")),
    };
    let fields_named = match data.fields {
        Fields::Named(f) => f.named,
        _ => return Err(syn::Error::new_spanned(name, "Only named fields supported")),
    };

    // Collect field info
    let mut known_keys_tokens: Vec<String> = Vec::new();
    let mut field_infos: Vec<FieldInfo> = Vec::new();

    for field in fields_named.iter() {
        let ident = field.ident.clone().unwrap();
        let ty = field.ty.clone();
        let attrs = parse_field_attributes(&field.attrs)?;
        let llsd_name = field_llsd_name(&ident, &attrs, &container_attrs);
        let is_option = is_type_option(&ty);
        if !attrs.skip && !attrs.flatten {
            known_keys_tokens.push(llsd_name.clone());
        }
        field_infos.push(FieldInfo {
            ident,
            attrs,
            llsd_name,
            is_option,
        });
    }

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let from_impl = match mode {
        Mode::From | Mode::Both => Some(gen_from(
            &field_infos,
            name,
            &impl_generics,
            &ty_generics,
            where_clause,
            &container_attrs,
        )),
        _ => None,
    };
    let into_impl = match mode {
        Mode::Into | Mode::Both => Some(gen_into(
            &field_infos,
            name,
            &impl_generics,
            &ty_generics,
            where_clause,
            &container_attrs,
        )),
        _ => None,
    };

    let from_tokens = from_impl.map(|body| {
        quote! { #body }
    });
    let into_tokens = into_impl.map(|body| {
        quote! { #body }
    });

    Ok(quote! { #from_tokens #into_tokens })
}

fn gen_from(
    fields: &[FieldInfo],
    name: &Ident,
    impl_generics: &impl ToTokens,
    ty_generics: &impl ToTokens,
    where_clause: Option<&syn::WhereClause>,
    container_attrs: &ContainerAttributes,
) -> proc_macro2::TokenStream {
    let deny_unknown = container_attrs.deny_unknown_fields;

    // Keys we consider known (exclude skip + flatten)
    let known_key_literals: Vec<proc_macro2::TokenStream> = fields
        .iter()
        .filter(|f| !f.attrs.skip && !f.attrs.flatten)
        .map(|f| {
            let k = &f.llsd_name;
            quote! { #k }
        })
        .collect();

    // Build per-field initialization expressions
    let mut field_inits: Vec<proc_macro2::TokenStream> = Vec::new();

    for f in fields {
        let ident = &f.ident;

        // Skip or skip_deserializing => just supply default
        if f.attrs.skip || f.attrs.skip_deserializing {
            let default_expr = match &f.attrs.default {
                DefaultType::None | DefaultType::Default => {
                    quote! { ::core::default::Default::default() }
                }
                DefaultType::Path(p) => quote! { #p() },
            };
            field_inits.push(quote! { #ident: #default_expr });
            continue;
        }

        // Flatten just delegates a full conversion from the whole value
        if f.attrs.flatten {
            field_inits.push(quote! { #ident: ::core::convert::TryFrom::try_from(llsd)? });
            continue;
        }

        let key = &f.llsd_name;
        let with_path = f.attrs.with.as_ref();

        let init_expr = if f.is_option {
            // Option fields
            match &f.attrs.default {
                DefaultType::None => {
                    if let Some(p) = with_path {
                        quote! { map.get(#key).map(|v| #p::deserialize(v)).transpose()? }
                    } else {
                        quote! { map.get(#key).map(|v| ::core::convert::TryFrom::try_from(v)).transpose()? }
                    }
                }
                DefaultType::Default => {
                    if let Some(p) = with_path {
                        quote! { map.get(#key).map(|v| #p::deserialize(v)).transpose()? }
                    } else {
                        quote! { map.get(#key).map(|v| ::core::convert::TryFrom::try_from(v)).transpose()? }
                    }
                }
                DefaultType::Path(func) => {
                    if let Some(p) = with_path {
                        quote! { map.get(#key).map(|v| #p::deserialize(v)).transpose()?.or_else(|| Some(#func())) }
                    } else {
                        quote! { map.get(#key).map(|v| ::core::convert::TryFrom::try_from(v)).transpose()?.or_else(|| Some(#func())) }
                    }
                }
            }
        } else {
            // Non-option fields
            match &f.attrs.default {
                DefaultType::None => {
                    if let Some(p) = with_path {
                        quote! {{
                            let raw = map.get(#key).ok_or_else(|| anyhow::Error::msg(format!("Missing required field: {}", #key)))?;
                            #p::deserialize(raw)?
                        }}
                    } else {
                        quote! { map.get(#key).ok_or_else(|| anyhow::Error::msg(format!("Missing required field: {}", #key)))?.try_into()? }
                    }
                }
                DefaultType::Default => {
                    if let Some(p) = with_path {
                        quote! { map.get(#key).map(|v| #p::deserialize(v)).transpose()?.unwrap_or_default() }
                    } else {
                        quote! { map.get(#key).map(|v| v.try_into()).transpose()?.unwrap_or_default() }
                    }
                }
                DefaultType::Path(func) => {
                    if let Some(p) = with_path {
                        quote! { map.get(#key).map(|v| #p::deserialize(v)).transpose()?.unwrap_or_else(|| #func()) }
                    } else {
                        quote! { map.get(#key).map(|v| v.try_into()).transpose()?.unwrap_or_else(|| #func()) }
                    }
                }
            }
        };

        field_inits.push(quote! { #ident: #init_expr });
    }

    quote! {
        impl #impl_generics ::core::convert::TryFrom<&llsd_rs::Llsd> for #name #ty_generics #where_clause {
            type Error = anyhow::Error;
            fn try_from(llsd: &llsd_rs::Llsd) -> ::core::result::Result<Self, Self::Error> {
                if let Some(map) = llsd.as_map() {
                    if #deny_unknown {
                        for key in map.keys() {
                            if !( #( key == #known_key_literals )||* ) {
                                return Err(anyhow::Error::msg(format!("Unknown field: {}", key)));
                            }
                        }
                    }
                    Ok(Self { #( #field_inits ),* })
                } else {
                    Err(anyhow::Error::msg("Expected LLSD Map"))
                }
            }
        }
        impl #impl_generics ::core::convert::TryFrom<llsd_rs::Llsd> for #name #ty_generics #where_clause {
            type Error = anyhow::Error;
            fn try_from(llsd: llsd_rs::Llsd) -> ::core::result::Result<Self, Self::Error> {
                <Self as ::core::convert::TryFrom<&llsd_rs::Llsd>>::try_from(&llsd)
            }
        }
    }
}
fn gen_into(
    fields: &[FieldInfo],
    name: &Ident,
    impl_generics: &impl ToTokens,
    ty_generics: &impl ToTokens,
    where_clause: Option<&syn::WhereClause>,
    _container_attrs: &ContainerAttributes,
) -> proc_macro2::TokenStream {
    let mut inserts = Vec::new();
    let idents: Vec<Ident> = fields.iter().map(|f| f.ident.clone()).collect();
    for f in fields {
        if f.attrs.skip || f.attrs.skip_serializing {
            continue;
        }
        let ident = &f.ident;
        let key = &f.llsd_name;
        let with_path = f.attrs.with.as_ref();
        let expr = match (f.is_option, f.attrs.flatten, with_path) {
            (true, _, Some(path)) => {
                quote! { if let Some(field_value) = #ident { map.insert(#key.to_string(), #path::serialize(&field_value)); } }
            }
            (true, _, None) => {
                quote! { if let Some(field_value) = #ident { map.insert(#key.to_string(), llsd_rs::Llsd::from(field_value)); } }
            }
            (false, true, Some(path)) => {
                quote! { if let llsd_rs::Llsd::Map(inner) = #path::serialize(&#ident) { for (k,v) in inner { map.insert(k, v); } } }
            }
            (false, true, None) => {
                quote! { if let llsd_rs::Llsd::Map(inner) = llsd_rs::Llsd::from(#ident) { for (k,v) in inner { map.insert(k, v); } } }
            }
            (false, false, Some(path)) => {
                quote! { map.insert(#key.to_string(), #path::serialize(&#ident)); }
            }
            (false, false, None) => {
                quote! { map.insert(#key.to_string(), llsd_rs::Llsd::from(#ident)); }
            }
        };
        inserts.push(expr);
    }
    quote! {
        impl #impl_generics ::core::convert::From<#name #ty_generics> for llsd_rs::Llsd #where_clause {
            fn from(value: #name #ty_generics) -> Self {
                let #name { #( #idents ),* } = value;
                let mut map = ::std::collections::HashMap::new();
                #(#inserts)*
                llsd_rs::Llsd::Map(map)
            }
        }
    }
}

// Utilities -----------------------------------------------------------------------------------
fn field_llsd_name(
    ident: &Ident,
    fattrs: &FieldAttributes,
    cattrs: &ContainerAttributes,
) -> String {
    if let Some(r) = &fattrs.rename {
        r.clone()
    } else if let Some(rule) = cattrs.rename_all {
        rule.apply(&ident.to_string())
    } else {
        ident.to_string()
    }
}
fn is_type_option(ty: &Type) -> bool {
    if let Type::Path(p) = ty
        && p.qself.is_none()
        && let Some(seg) = p.path.segments.first()
    {
        return seg.ident == "Option";
    }
    false
}
fn to_snake_case(s: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for ch in s.chars() {
        if ch.is_uppercase() {
            if prev_lower {
                out.push('_');
            }
            for l in ch.to_lowercase() {
                out.push(l);
            }
            prev_lower = false;
        } else {
            out.push(ch);
            prev_lower = true;
        }
    }
    out
}
fn to_camel_case(s: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in s.chars() {
        if ch == '_' || ch == '-' {
            upper = true;
            continue;
        }
        if upper {
            for u in ch.to_uppercase() {
                out.push(u);
            }
            upper = false;
        } else {
            out.push(ch.to_ascii_lowercase());
        }
    }
    out
}
fn to_pascal_case(s: &str) -> String {
    let camel = to_camel_case(s);
    let mut chars = camel.chars();
    if let Some(f) = chars.next() {
        f.to_uppercase().collect::<String>() + chars.as_str()
    } else {
        String::new()
    }
}
