use proc_macro2::{Ident, Literal};
use simd_json::OwnedValue;
use simd_json::prelude::*;
use syn::{LitStr, parse::{Parse, ParseStream}, Path, Variant};
use syn::{Attribute, Field, Token};

#[derive(Debug, Default)]
pub(crate) struct FieldAttrs {
    rename: Option<String>,
    skip_serializing_if: Option<Path>,
    default: bool,
}

impl Parse for FieldAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = FieldAttrs::default();

        while !input.is_empty() {
            let attr: Ident = input.parse()?;
            match attr.to_string().as_str() {
                "rename" => {
                    let _equal_token: Token![=] = input.parse()?;
                    let name: LitStr = input.parse()?;

                    attrs.rename = Some(name.value());
                }
                "skip_serializing_if" => {
                    let _equal_token: Token![=] = input.parse()?;
                    let function: LitStr = input.parse()?;

                    let path: Path = function.parse()?;

                    attrs.skip_serializing_if = Some(path);
                }
                "default" => {
                    attrs.default = true;
                }
                "borrow" => (),
                other => {
                    return Err(syn::Error::new(
                        attr.span(),
                        format!("unexpected attribute `{}`", other),
                    ));
                }
            }
            if !input.is_empty() {
                let _comma_token: Token![,] = input.parse()?;
            }
        }
        Ok(attrs)
    }
}

#[derive(Debug, Default)]
pub(crate) struct VariantAttrs {
    rename: Option<String>,
}

impl Parse for VariantAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = VariantAttrs::default();

        while !input.is_empty() {
            let attr: Ident = input.parse()?;
            match attr.to_string().as_str() {
                "rename" => {
                    let _equal_token: Token![=] = input.parse()?;
                    let name: LitStr = input.parse()?;

                    attrs.rename = Some(name.value());
                }
                other => {
                    return Err(syn::Error::new(
                        attr.span(),
                        format!("unexpected attribute `{}`", other),
                    ));
                }
            }
            if !input.is_empty() {
                let _comma_token: Token![,] = input.parse()?;
            }
        }
        Ok(attrs)
    }
}

#[derive(Debug)]
pub(crate) enum RenameAll {
    None,
    CamelCase,
    Lowercase,
}

fn capitalize(field: &str) -> String {
    let mut chars = field.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

impl RenameAll {
    fn apply(&self, field: &str) -> String {
        match self {
            RenameAll::None => String::from(field),
            RenameAll::Lowercase => field.to_lowercase(),
            RenameAll::CamelCase => {
                let mut parts = field.split('_');
                let first = parts.next().expect("zero length name");
                format!(
                    "{}{}",
                    first,
                    parts.map(capitalize).collect::<Vec<String>>().join("")
                )
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct StructAttrs {
    rename_all: RenameAll,
    deny_unknown_fields: bool,
    default: bool,
}

impl Default for StructAttrs {
    fn default() -> Self {
        StructAttrs {
            rename_all: RenameAll::None,
            deny_unknown_fields: false,
            default: false,
        }
    }
}

impl Parse for StructAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut rename_all = RenameAll::None;
        let mut deny_unknown_fields = false;
        let mut default = false;
        while !input.is_empty() {
            let attr: Ident = input.parse()?;
            match attr.to_string().as_str() {
                "rename_all" => {
                    let _equal_token: Token![=] = input.parse()?;
                    let name: Literal = input.parse()?;

                    match name.to_string().as_str() {
                        r#""camelCase""# => rename_all = RenameAll::CamelCase,
                        r#""lowercase""# => rename_all = RenameAll::Lowercase,
                        other => {
                            return Err(syn::Error::new(
                                attr.span(),
                                format!("unexpected rename_all type `{}`", other),
                            ));
                        }
                    }
                }
                "deny_unknown_fields" => {
                    deny_unknown_fields = true;
                }
                "default" => {
                    default = true;
                }
                other => {
                    return Err(syn::Error::new(
                        attr.span(),
                        format!("unexpected field attribute `{}`", other),
                    ));
                }
            }
            if !input.is_empty() {
                let _comma_token: Token![,] = input.parse()?;
            }
        }
        Ok(StructAttrs {
            rename_all,
            deny_unknown_fields,
            default,
        })
    }
}

pub fn field_attrs(attr: &Attribute) -> FieldAttrs {
    attr.parse_args::<FieldAttrs>()
        .expect("failed to parse attributes")
}

pub(crate) fn variant_attrs(attr: &Attribute) -> VariantAttrs {
    attr.parse_args::<VariantAttrs>()
        .expect("failed to parse attributes")
}

pub fn struct_attrs(attr: &Attribute) -> StructAttrs {
    attr.parse_args::<StructAttrs>()
        .expect("failed to parse attributes")
}

pub fn get_attr<'field>(attrs: &'field [Attribute], name: &str) -> Option<&'field Attribute> {
    attrs
        .iter()
        .find(|a| a.path().get_ident().map(|i| i == name).unwrap_or_default())
}

impl StructAttrs {
    pub(crate) fn parse(attrs: Vec<Attribute>) -> StructAttrs {
        if let Some(attrs) = get_attr(&attrs, "simd_json") {
            struct_attrs(attrs)
        } else if let Some(attrs) = get_attr(&attrs, "serde") {
            struct_attrs(attrs)
        } else {
            StructAttrs::default()
        }
    }
    pub(crate) fn deny_unknown_fields(&self) -> bool {
        self.deny_unknown_fields
    }

    pub(crate) fn skip_serializing_if(&self, field: &Field) -> Option<Path> {
        get_attr(&field.attrs, "simd_json")
            .or_else(|| get_attr(&field.attrs, "serde"))
            .map(field_attrs)
            .and_then(|a| a.skip_serializing_if)
    }
    
    pub(crate) fn name_field(&self, field: &Field) -> String {
        if let Some(attr) = get_attr(&field.attrs, "simd_json")
            .map(field_attrs)
            .and_then(|a| a.rename)
        {
            format!("{}:", OwnedValue::from(attr).encode())
        } else if let Some(attr) = get_attr(&field.attrs, "serde")
            .map(field_attrs)
            .and_then(|a| a.rename)
        {
            format!("{}:", OwnedValue::from(attr).encode())
        } else {
            let f = field
                .ident
                .as_ref()
                .expect("Field is missing ident")
                .to_string();
            format!("{}:", OwnedValue::from(self.rename_all.apply(&f)).encode())
        }
    }

    pub(crate) fn name_variant(&self, variant: &Variant) -> String {
        if let Some(attr) = get_attr(&variant.attrs, "simd_json")
            .map(variant_attrs)
            .and_then(|a| a.rename)
        {
            format!("{}", attr)
        } else if let Some(attr) = get_attr(&variant.attrs, "serde")
            .map(variant_attrs)
            .and_then(|a| a.rename)
        {
            format!("{}", attr)
        } else {
            let v = variant.ident.to_string();
            format!("{}", self.rename_all.apply(&v))
        }
    }
    
    pub(crate) fn default_fallback(&self, field: &Field) -> bool {
        if self.default {
            true
        } else if let Some(attr) = get_attr(&field.attrs, "simd_json")
            .map(field_attrs)
            .and_then(|a| Some(a.default))
        {
            attr
        } else if let Some(attr) = get_attr(&field.attrs, "serde")
            .map(field_attrs)
            .and_then(|a| Some(a.default))
        {
            attr
        } else {
            false
        }
    }
}
