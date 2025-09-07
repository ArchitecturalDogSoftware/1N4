// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2025 RemasteredArch
//
// This file is part of 1N4.
//
// 1N4 is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// 1N4 is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License along with 1N4. If not, see
// <https://www.gnu.org/licenses/>.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::panic,
    clippy::missing_docs_in_private_items,
    reason = "still experimenting"
)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Error, Expr, ExprArray, Field, Fields, FieldsNamed, FieldsUnnamed, Ident,
    Meta, MetaList, MetaNameValue, Path, Result, Token, Type, parse_macro_input,
};

/// Hardcoded [`Path`]s representing various annotations.
mod attr_paths {
    use super::{Ident, Path, Punctuated, Span};

    /// Return a [`Path`] representing the `#[doc(...)]` macro.
    pub fn doc() -> Path {
        Path {
            leading_colon: None,
            segments: {
                let mut punctuated = Punctuated::new();
                punctuated.push_value(Ident::new("doc", Span::mixed_site()).into());
                punctuated
            },
        }
    }

    /// Return a [`Path`] representing the `#[derive(...)]` macro.
    pub fn derive() -> Path {
        Path {
            leading_colon: None,
            segments: {
                let mut punctuated = Punctuated::new();
                punctuated.push_value(Ident::new("derive", Span::mixed_site()).into());
                punctuated
            },
        }
    }

    /// Return a [`Path`] representing the `#[serde(...)]` annotation.
    // TO-DO: does this need to be replaced this with a qualified path?
    pub fn serde() -> Path {
        Path {
            leading_colon: None,
            segments: {
                let mut punctuated = Punctuated::new();
                punctuated.push_value(Ident::new("serde", Span::mixed_site()).into());
                punctuated
            },
        }
    }

    /// Return a [`Path`] representing the `#[option(...)]` annotation.
    pub fn option() -> Path {
        Path {
            leading_colon: None,
            segments: {
                let mut punctuated = Punctuated::new();
                punctuated.push_value(Ident::new("option", Span::mixed_site()).into());
                punctuated
            },
        }
    }

    /// Return a [`Path`] representing the `#[arg(...)]` annotation.
    pub fn arg() -> Path {
        Path {
            leading_colon: None,
            segments: {
                let mut punctuated = Punctuated::new();
                punctuated.push_value(Ident::new("arg", Span::mixed_site()).into());
                punctuated
            },
        }
    }
}

#[derive(Debug)]
struct OptionalArguments {
    keep_derives: Option<Vec<Path>>,
    keep_annotations: Vec<Path>,
    keep_field_annotations: Vec<Path>,
    attr_span: Span,
}

impl Parse for OptionalArguments {
    fn parse(input: ParseStream) -> Result<Self> {
        fn parse_paths(bracketed_list: &Expr) -> Result<Vec<Path>> {
            let Expr::Array(ExprArray { elems: exprs, .. }) = bracketed_list else {
                return Err(Error::new(bracketed_list.span(), "unknown value, expected bracketed list"));
            };

            let mut paths = Vec::with_capacity(exprs.len());
            for elem in exprs {
                match elem {
                    Expr::Path(path_expr) => paths.push(path_expr.path.clone()),
                    _ => return Err(Error::new(elem.span(), "unknown value, expected path")),
                }
            }

            Ok(paths)
        }

        let arguments = Punctuated::<MetaNameValue, Token![,]>::parse_terminated(input)?;
        let mut keep_derives = None;
        let mut keep_annotations = None;
        let mut keep_field_annotations = None;

        for MetaNameValue { path, value, .. } in arguments {
            match path.require_ident()?.to_string().as_str() {
                "keep_derives" => keep_derives = Some(parse_paths(&value)?),
                "keep_annotations" => keep_annotations = Some(parse_paths(&value)?),
                "keep_field_annotations" => keep_field_annotations = Some(parse_paths(&value)?),
                other => return Err(Error::new(path.span(), format!("unknown argument: {other}"))),
            }
        }

        let mut keep_annotations = keep_annotations.unwrap_or_else(|| Vec::with_capacity(1));
        keep_annotations.push(attr_paths::doc());
        if keep_derives.is_some() {
            keep_annotations.push(attr_paths::derive());
        }

        let mut keep_field_annotations = keep_field_annotations.unwrap_or_else(|| Vec::with_capacity(1));
        keep_field_annotations.push(attr_paths::doc());

        Ok(Self { keep_derives, keep_annotations, keep_field_annotations, attr_span: input.span() })
    }
}

struct DefaultEqExpr {
    /// The `default` token. Not actually an [`Ident`], but it's good enough.
    default: Ident,
    eq: Token![=],
    expr: Expr,
    span: Span,
}

impl Parse for DefaultEqExpr {
    fn parse(input: ParseStream) -> Result<Self> {
        let span = input.span();

        let default = input.parse()?;
        let eq = input.parse()?;
        let expr = input.parse()?;

        Ok(Self { default, eq, expr, span })
    }
}

struct FieldsWithDefaults {
    /// The [`Ident`] of the struct with non-[optional] fields.
    ///
    /// [optional]: `Option`
    ident: Ident,
    /// The [`Ident`] of the struct with [optional] fields.
    ///
    /// [optional]: `Option`
    optional_ident: Ident,
    fields: Vec<FieldWithDefault>,
}

impl FieldsWithDefaults {
    fn generate_conversions(&self) -> TokenStream {
        let Self { ident, optional_ident, fields } = self;

        let idents = fields.iter().map(|FieldWithDefault { ident, .. }| ident).collect::<Vec<_>>();
        let assign_unwrap_or_default = fields
            .iter()
            .map(|FieldWithDefault { ident, default }| {
                quote! {
                    #ident: self.#ident.unwrap_or_else(|| #default)
                }
            })
            .collect::<Vec<_>>();

        quote! {
            impl ::std::convert::From<#ident> for #optional_ident {
                fn from(value: #ident) -> Self {
                    Self {
                        #( #idents: Some(value.#idents) ),*
                    }
                }
            }

            impl #optional_ident {
                #[doc = ::std::concat!(
                    "Create a new [`",
                    ::std::stringify!( #ident ),
                    "`] by filling all [`None`][`::std::option::Option::None`] values with the provided default function.",
                )]
                pub fn fill_defaults(self) -> #ident {
                    #ident {
                        #( #assign_unwrap_or_default ),*
                    }
                }
            }
        }
        .into()
    }
}

struct FieldWithDefault {
    ident: Ident,
    default: Expr,
}

impl FieldWithDefault {
    /// Create a new [`Self`] from an arbitrary [`Field`].
    ///
    /// # Errors
    ///
    /// Returns an error if the provided [`Field`] does not have the `#[option(...)]` annotation or
    /// if it is malformed.
    fn new(field: &Field) -> Result<Self> {
        let option_attr_path = attr_paths::option();
        let Some(mut option_attr) = field.attrs.iter().find(|attr| attr.path() == &option_attr_path).cloned() else {
            return Err(Error::new(field.span(), "missing `#[option(...)]` annotation to provide default values"));
        };

        let Some(ident) = field.ident.clone() else {
            todo!("implement support for tuple structs");
        };

        let default: Expr = match &mut option_attr.meta {
            // Of the form `#[option(default = EXPRESSION)]`.
            Meta::NameValue(meta_name_value) => meta_name_value.value.clone(),
            // Of the form `#[option(default)]`.
            Meta::List(list)
                if syn::parse::<Ident>(list.tokens.clone().into()).is_ok_and(|ident| ident == "default") =>
            {
                syn::parse(quote! { Default::default() }.into()).unwrap()
            }
            // Also of the form `#[option(default = EXPRESSION)]`.
            //
            // For some reason, this is what triggers for `#[option(default = self::default_status_file())]`.
            Meta::List(list) => {
                let DefaultEqExpr { expr, .. } = syn::parse(list.tokens.clone().into()).map_err(|_| {
                    Error::new(
                        field.span(),
                        "expected annotation in the form of `#[option(default)]` or `#[option(default = EXPRESSION)]`",
                    )
                })?;

                expr
            }
            // Of another form.
            other => {
                dbg!(other);
                return Err(Error::new(
                    field.span(),
                    "expected annotation in the form of `#[option(default)]` or `#[option(default = EXPRESSION)]`",
                ));
            }
        };

        Ok(Self { ident, default })
    }
}

/// Applies the procedural macro.
///
/// Input should be shaped like:
///
/// ```rust
/// #[optional(
///     keep_derives = [Clone, Debug, Hash, PartialEq, Eq, Args],
///     keep_annotations = [non_exhaustive, expect],
/// )]
/// ```
pub fn procedure(attribute_args: TokenStream, item: TokenStream) -> TokenStream {
    let arguments = parse_macro_input!(attribute_args as OptionalArguments);

    let DeriveInput { attrs, ident, generics, vis, data } = parse_macro_input!(item as DeriveInput);
    let Data::Struct(DataStruct { struct_token, mut fields, semi_token: semicolon_token }) = data else {
        return Error::new(arguments.attr_span, "`optional` only supports structs")
            .to_compile_error()
            .to_token_stream()
            .into();
    };

    let optional_ident = format_ident!("Optional{ident}");

    let mut fields_with_defaults = FieldsWithDefaults {
        ident: ident.clone(),
        optional_ident: optional_ident.clone(),
        fields: Vec::with_capacity(fields.len()),
    };

    let optional_fields = self::fields_to_optional(fields.clone());

    for field in &mut fields {
        fields_with_defaults.fields.push(match FieldWithDefault::new(field) {
            Ok(with_default) => with_default,
            Err(error) => return error.to_compile_error().into_token_stream().into(),
        });

        field.attrs.retain(|attr| {
            let path = attr.path();

            // Naive search, because I don't care.
            arguments.keep_field_annotations.iter().any(|kept_path| path == kept_path)
        });
    }

    let conversions: proc_macro2::TokenStream = fields_with_defaults.generate_conversions().into();

    let optional_attrs = attrs;
    let attrs = self::only_kept_attrs(&optional_attrs, &arguments.keep_annotations, arguments.keep_derives.as_deref());

    quote! {
        use ::clap::builder::TypedValueParser;

        #( #optional_attrs )*
        #vis #struct_token #optional_ident #generics
        #optional_fields
        #semicolon_token

        #( #attrs )*
        #vis #struct_token #ident #generics
        #fields
        #semicolon_token

        #conversions
    }
    .into()
}

fn fields_to_optional(fields: Fields) -> Fields {
    match fields {
        Fields::Named(FieldsNamed { brace_token, named }) => {
            Fields::Named(FieldsNamed { brace_token, named: named.into_iter().map(field_to_optional).collect() })
        }
        Fields::Unnamed(FieldsUnnamed { paren_token, unnamed }) => Fields::Unnamed(FieldsUnnamed {
            paren_token,
            unnamed: unnamed.into_iter().map(field_to_optional).collect(),
        }),
        Fields::Unit => Fields::Unit,
    }
}

fn field_to_optional(Field { mut attrs, vis, mutability, ident, colon_token, ty }: Field) -> Field {
    let option_attr_path = attr_paths::option();
    if let Some(option_attr) = attrs.iter_mut().find(|attr| attr.path() == &option_attr_path) {
        option_attr.meta = Meta::List(MetaList {
            path: attr_paths::serde(),
            delimiter: option_attr.meta.require_list().unwrap().delimiter.clone(),
            tokens: quote! {
                default = "::std::option::Option::default", skip_serializing_if = "::std::option::Option::is_none"
            },
        });
    }

    attrs.retain(|attr| attr.path() != &option_attr_path);

    Field {
        attrs,
        vis,
        mutability,
        ident,
        colon_token,
        ty: Type::Path(
            syn::parse(
                quote! {
                    // This must be `Option<T>`, not `::std::option::Option<T>`, because Clap
                    // currently matches on `Option` (to infer that an argument is optional) on a
                    // strictly textual basis, it doesn't attempt to infer from the actual type.
                    //
                    // See:
                    //
                    // - <https://github.com/clap-rs/clap/issues/4636#issuecomment-1381969663>
                    // - <https://github.com/clap-rs/clap/issues/4626>
                    Option< #ty >
                }
                .into(),
            )
            .unwrap(),
        ),
    }
}

fn filter_fields_attrs(fields: &mut Fields, kept_attrs: &[Path]) {
    for field in fields {
        field.attrs = self::only_kept_attrs(&field.attrs, kept_attrs, None);
    }
}

fn only_doc_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    self::only_kept_attrs(attrs, &[attr_paths::doc()], None)
}

fn only_kept_attrs(attrs: &[Attribute], kept_attrs: &[Path], keep_derives: Option<&[Path]>) -> Vec<Attribute> {
    let derive_annotation_path = attr_paths::derive();

    attrs
        .iter()
        .filter(|attr| {
            let path = attr.meta.path();

            // Naive search, because I don't care.
            kept_attrs.iter().any(|kept_path| path == kept_path)
        })
        .cloned()
        .map(|mut attr| {
            let Some(keep_derives) = keep_derives else {
                return attr;
            };
            if attr.meta.path() != &derive_annotation_path {
                return attr;
            }

            let Meta::List(list) = &mut attr.meta else {
                panic!("`derive` macros should always have lists");
            };
            list.tokens = quote! { #( #keep_derives ),* };

            attr
        })
        .collect()
}
