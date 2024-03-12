use proc_macro::{self, TokenStream};
use proc_macro2::{Ident, Span};
use quote::quote;
use simd_json::prelude::*;
use syn::token::Comma;
use syn::{
    parse_macro_input, punctuated::Punctuated, Data, DataEnum, DataStruct, DeriveInput, Field,
    Fields, FieldsNamed, FieldsUnnamed, Generics, Variant,
};

use crate::args::*;

/// Unnamed struct as `Struct(u8)` or `Struct(u8, String)`
fn derive_unnamed_struct(
    ident: Ident,
    generics: Generics,
    fields: Punctuated<Field, Comma>,
) -> proc_macro::TokenStream {
    if fields.len() == 1 {
        let expanded = quote! {
            impl #generics simd_json_derive::Serialize for #ident #generics {
                fn json_write<W>(&self, writer: &mut W) -> std::io::Result<()>
                where
                    W: std::io::Write {
                        self.0.json_write(writer)
                    }
            }
        };
        TokenStream::from(expanded)
    } else {
        let keys: Vec<_> = fields
            .iter()
            .enumerate()
            .map(|(i, _)| syn::Index::from(i))
            .skip(1)
            .collect();
        let expanded = quote! {
            impl #generics simd_json_derive::Serialize for #ident #generics {
                fn json_write<W>(&self, writer: &mut W) -> std::io::Result<()>
                where
                    W: std::io::Write {
                        writer.write_all(b"[")?;
                        self.0.json_write(writer)?;
                        #(
                            writer.write_all(b",")?;
                            self.#keys.json_write(writer)?;
                        )*
                        writer.write_all(b"]")
                    }
            }
        };
        TokenStream::from(expanded)
    }
}

/// Named struct as `Struct(u8)` or `Struct(u8, String)`
fn derive_named_struct(
    attrs: StructAttrs,
    ident: Ident,
    generics: Generics,
    fields: Punctuated<Field, Comma>,
) -> proc_macro::TokenStream {
    let mut keys = Vec::new();
    let mut values = Vec::new();
    let mut skip_if = Vec::new();

    for f in &fields {
        let ident = f.ident.clone().expect("Missing ident");
        let name = attrs.name_field(f);
        keys.push(name);
        values.push(ident);
        skip_if.push(attrs.skip_serializing_if(f));
    }
    let expanded = if skip_if.iter().all(Option::is_none) {
        if let Some((first, rest)) = keys.split_first_mut() {
            *first = format!("{{{}", first);
            for r in rest {
                *r = format!(",{}", r);
            }
        };

        quote! {
            impl #generics simd_json_derive::Serialize for #ident #generics {
                #[inline]
                fn json_write<W>(&self, writer: &mut W) -> std::io::Result<()>
                where
                    W: std::io::Write {
                        #(
                            writer.write_all(#keys.as_bytes())?;
                            self.#values.json_write(writer)?;
                        )*
                        writer.write_all(b"}")
                    }
            }
        }
    } else {
        let writes = keys
            .iter()
            .zip(values.iter())
            .zip(skip_if.iter())
            .map(|((k, v), s)| {
                if let Some(s) = s {
                    quote! {
                        if !#s(&self.#v) {
                            if has_written_key {
                                writer.write_all(b",")?;
                            }
                            has_written_key = true;
                            writer.write_all(#k.as_bytes())?;
                            self.#v.json_write(writer)?;
                        }
                    }
                } else {
                    quote! {
                        if has_written_key {
                            writer.write_all(b",")?;
                        }
                        has_written_key = true;
                        writer.write_all(#k.as_bytes())?;
                        self.#v.json_write(writer)?;
                    }
                }
            })
            .collect::<Vec<_>>();
        quote! {
            impl #generics simd_json_derive::Serialize for #ident #generics {
                #[inline]
                fn json_write<W>(&self, writer: &mut W) -> std::io::Result<()>
                where
                    W: std::io::Write {
                        writer.write_all(b"{")?;
                        let mut has_written_key = false;
                        #(
                            #writes
                        )*
                        writer.write_all(b"}")
                    }
            }
        }
    };
    TokenStream::from(expanded)
}

fn derive_enum(
    attrs: StructAttrs,
    ident: Ident,
    data: DataEnum,
    generics: Generics,
) -> TokenStream {
    let mut body_elements = Vec::new();
    let variants = data.variants;
    let (simple, variants): (Vec<_>, Vec<_>) =
        variants.into_iter().partition(|v| v.fields.is_empty());
    let (named, unnamed): (Vec<_>, Vec<_>) = variants.iter().partition(|v| {
        matches!(
            v,
            Variant {
                fields: Fields::Named(_),
                ..
            }
        )
    });

    let (unnamed1, unnamed): (Vec<_>, Vec<_>) =
        unnamed.into_iter().partition(|v| v.fields.len() == 1);

    // enum no fields of Enum::Variant
    // They serialize as: "Variant"

    let (simple_keys, simple_values): (Vec<_>, Vec<_>) = simple
        .iter()
        .map(|s| {
            (
                &s.ident,
                simd_json::OwnedValue::from(attrs.name_variant(&s)).encode(),
            )
        })
        .unzip();
    let simple = quote! {
        #(
            #ident::#simple_keys => writer.write_all(#simple_values.as_bytes())
        ),*
    };

    if !simple.is_empty() {
        body_elements.push(simple);
    }

    // Unnamed enum variants with exactly 1 field of Enum::Variant(type1)
    // They serialize as: {"Varriant":..}

    let (unnamed1_idents, unnamed1_keys): (Vec<_>, Vec<_>) = unnamed1
        .iter()
        .map(|v| {
            (
                &v.ident,
                format!(
                    "{{{}:",
                    simd_json::OwnedValue::from(attrs.name_variant(&v)).encode()
                ),
            )
        })
        .unzip();
    let unnamed1 = quote! {
        #(
            #ident::#unnamed1_idents(v) => {
                writer.write_all(#unnamed1_keys.as_bytes())?;
                v.json_write(writer)?;
                writer.write_all(b"}")
            }
        ),*
    };
    if !unnamed1.is_empty() {
        body_elements.push(unnamed1);
    }

    // Unnamed enum variants with more then 1 field of Enum::Variant(type1, type2, type3)
    // They serialize as: {"Varriant":[.., .., ..]}

    let (unnamed_ident_and_vars, unnamed_keys): (Vec<_>, Vec<_>) = unnamed
        .iter()
        .map(|v| {
            (
                (
                    &v.ident,
                    (0..v.fields.len())
                        .map(|i| Ident::new(&format!("v{}", i), Span::call_site()))
                        .collect::<Vec<_>>(),
                ),
                format!(
                    "{{{}:[",
                    simd_json::OwnedValue::from(attrs.name_variant(&v)).encode()
                ),
            )
        })
        .unzip();

    let (unnamed_idents, unnamed_var_names): (Vec<_>, Vec<_>) =
        unnamed_ident_and_vars.into_iter().unzip();

    let unnamed_vecs = unnamed_var_names.iter().map(|vs| {
        let (first, rest) = vs.split_first().expect("zero unnamed vars");
        quote! {
            #first.json_write(writer)?;
            #(
                writer.write_all(b",")?;
                #rest.json_write(writer)?;
            )*
        }
    });

    let unnamed_vars = unnamed_var_names.iter().map(|vs| quote! { #(#vs),* });

    let unnamed = quote! {
        #(
            #ident::#unnamed_idents(#unnamed_vars) =>
            {
                writer.write_all(#unnamed_keys.as_bytes())?;
                #unnamed_vecs
                writer.write_all(b"]}")
            }
        ),*
    };
    if !unnamed.is_empty() {
        body_elements.push(unnamed);
    }

    // Named enum variants of the form Enum::Variant{key1: type, key2: type...}
    // They serialize as: {"Varriant":{"key1":..,"key2":..}}

    let mut named_bodies = Vec::new();
    for v in named {
        let named_ident = &v.ident;
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut skip_if = Vec::new();

        for f in &v.fields {
            let name = attrs.name_field(f);
            let ident = f.ident.clone().expect("Missing ident");
            keys.push(name);
            values.push(ident);
            skip_if.push(attrs.skip_serializing_if(f));
        }
        let variant_name = simd_json::OwnedValue::from(attrs.name_variant(&v)).encode();

        named_bodies.push(if skip_if.iter().all(Option::is_none) {
            let (first_key, rest_keys) = keys.split_first().expect("zero fields");
            let (first_value, rest_values) = values.split_first().expect("zero fields");

            let start = format!("{{{variant_name}:{{{first_key}",);
            let rest_keys = rest_keys
                .iter()
                .map(|k| format!(",{k}"))
                .collect::<Vec<_>>();

            quote! {
                #ident::#named_ident{#(#values),*} => {
                    writer.write_all(#start.as_bytes())?;
                    #first_value.json_write(writer)?;
                    #(
                        writer.write_all(#rest_keys.as_bytes())?;
                        #rest_values.json_write(writer)?;

                    )*
                    writer.write_all(b"}}")
                }
            }
        } else {
            let writes = keys
                .iter()
                .zip(values.iter())
                .zip(skip_if.iter())
                .map(|((k, v), s)| {
                    if let Some(s) = s {
                        quote! {

                            if !#s(#v) {
                                if has_written_key {
                                    writer.write_all(b",")?;
                                }
                                has_written_key = true;
                                writer.write_all(#k.as_bytes())?;
                                #v.json_write(writer)?;
                            }
                        }
                    } else {
                        quote! {
                            if has_written_key {
                                writer.write_all(b",")?;
                            }
                            has_written_key = true;
                            writer.write_all(#k.as_bytes())?;
                            #v.json_write(writer)?;
                        }
                    }
                })
                .collect::<Vec<_>>();
            let prefix = format!("{{{variant_name}:{{");
            quote! {
                #ident::#named_ident{#(#values),*} => {
                    writer.write_all(#prefix.as_bytes())?;
                    let mut has_written_key = false;
                    #(
                        #writes
                    )*
                    writer.write_all(b"}}")
                }
            }
        });
    }
    let named = quote! {#(#named_bodies),*};

    if !named.is_empty() {
        body_elements.push(named);
    }

    let match_body = quote! {
        #(#body_elements),*
    };

    let expanded = quote! {
        impl #generics simd_json_derive::Serialize for #ident #generics {
            #[inline]
            fn json_write<W>(&self, writer: &mut W) -> std::io::Result<()>
            where
                W: std::io::Write {
                    match self {
                        #match_body
                    }

                }
        }
    };
    TokenStream::from(expanded)
}

pub(crate) fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match input {
        // Unnamed struct
        DeriveInput {
            ident,
            data:
                Data::Struct(DataStruct {
                    fields: Fields::Unnamed(FieldsUnnamed { unnamed, .. }),
                    ..
                }),
            generics,
            ..
        } => derive_unnamed_struct(ident, generics, unnamed),
        // Named
        DeriveInput {
            ident,
            attrs,
            data:
                Data::Struct(DataStruct {
                    fields: Fields::Named(FieldsNamed { named, .. }),
                    ..
                }),
            generics,
            ..
        } => {
            let attrs = if let Some(attrs) = get_attr(&attrs, "simd_json") {
                struct_attrs(attrs)
            } else if let Some(attrs) = get_attr(&attrs, "serde") {
                struct_attrs(attrs)
            } else {
                StructAttrs::default()
            };
            derive_named_struct(attrs, ident, generics, named)
        }
        DeriveInput {
            ident,
            data: Data::Enum(data),
            generics,
            attrs,
            ..
        } => {
            let attrs = if let Some(attrs) = get_attr(&attrs, "simd_json") {
                struct_attrs(attrs)
            } else if let Some(attrs) = get_attr(&attrs, "serde") {
                struct_attrs(attrs)
            } else {
                StructAttrs::default()
            };
            derive_enum(attrs, ident, data, generics)
        }
        _ => TokenStream::from(quote! {}),
    }
}
