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
    // dead_code,
    clippy::unwrap_used,
    clippy::panic,
    clippy::missing_docs_in_private_items,
    reason = "still experimenting"
)]

use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Attribute, Data, DataStruct, DeriveInput, Error, Meta, Path, parse_macro_input};

/// Parse item-level annotation arguments.
mod arguments;
/// Hardcoded [`Path`]s representing various annotations.
mod attr_paths;
/// Parse fields and their annotation.
mod fields;

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
    let arguments = parse_macro_input!(attribute_args as arguments::OptionalArguments);

    let DeriveInput { attrs, ident, generics, vis, data } = parse_macro_input!(item as DeriveInput);
    let Data::Struct(DataStruct { struct_token, mut fields, semi_token: semicolon_token }) = data else {
        return Error::new(arguments.attr_span, "`optional` only supports structs")
            .to_compile_error()
            .to_token_stream()
            .into();
    };

    let optional_ident = format_ident!("Optional{ident}");

    let mut fields_with_defaults = fields::FieldsWithDefaults {
        ident: ident.clone(),
        optional_ident: optional_ident.clone(),
        fields: Vec::with_capacity(fields.len()),
    };

    let optional_fields = fields::fields_to_optional(fields.clone());

    for field in &mut fields {
        fields_with_defaults.fields.push(match fields::FieldWithDefault::new(field) {
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
    let mut attrs =
        self::only_kept_attrs(&optional_attrs, &arguments.keep_annotations, arguments.keep_derives.as_deref());
    if let Some(mut extra_annotations) = arguments.apply_annotations {
        attrs.append(&mut extra_annotations);
    }

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
