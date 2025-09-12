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
use syn::spanned::Spanned;
use syn::{Attribute, Data, DataStruct, DeriveInput, Error, parse_macro_input};

/// Parse item-level annotation arguments.
mod arguments;
/// Hardcoded [paths][`syn::Path`] representing various annotations.
mod attr_paths;
/// Parse fields and their annotation.
mod fields;

/// Prepends [`macro@Default`] to the first existing [`derive`] [`Attribute`], or appends a new [`derive`] [`Attribute`]
/// for just [`macro@Default`].
fn add_derive_default(attributes: &mut Vec<Attribute>) -> syn::Result<()> {
    let derive_annotation_path = &attr_paths::derive();

    if let Some(derive_attr) = attributes.iter_mut().find(|attr| attr.path() == derive_annotation_path) {
        let syn::Meta::List(list) = &mut derive_attr.meta else {
            return Err(Error::new(derive_attr.meta.span(), "received a non-list `derive` macro call"));
        };

        let rest = &list.tokens;
        list.tokens = quote! {
            ::std::default::Default, #rest
        };
    } else {
        attributes.push(syn::parse_quote! { #[derive(::std::default::Default)] });
    }

    Ok(())
}

// TO-DO: `<#field_type>::new()` instead of `#field_type::new()`.
/// Applies [the procedural macro][`macro@crate::optional`].
#[must_use]
pub fn procedure(attribute_args: TokenStream, item: TokenStream) -> TokenStream {
    let arguments = parse_macro_input!(attribute_args as arguments::OptionalArguments);

    let DeriveInput { attrs: input_attrs, ident: input_ident, generics, vis, data } =
        parse_macro_input!(item as DeriveInput);
    let Data::Struct(DataStruct { struct_token, fields: input_fields, semi_token: semicolon_token }) = data else {
        return Error::new(arguments.span(), "`optional` only supports structs")
            .to_compile_error()
            .to_token_stream()
            .into();
    };

    let optional_ident = format_ident!("Optional{input_ident}");
    let ident = input_ident;

    let mut fields_with_defaults = fields::FieldsWithDefaults {
        ident: ident.clone(),
        optional_ident: optional_ident.clone(),
        fields: Vec::with_capacity(input_fields.len()),
    };

    let optional_fields = fields::fields_to_optional(input_fields.clone());
    let mut fields = input_fields;

    for field in &mut fields {
        fields_with_defaults.fields.push(match fields::FieldWithDefault::new(field) {
            Ok(with_default) => with_default,
            Err(error) => return error.to_compile_error().into(),
        });

        arguments.retain_only_kept_field_attrs(field);
    }

    let conversions = fields_with_defaults.generate_conversions();

    let attrs = arguments.only_kept_and_applied_attrs(&input_attrs);
    let mut optional_attrs = input_attrs;
    if let Err(e) = self::add_derive_default(&mut optional_attrs) {
        return e.to_compile_error().into();
    }

    quote! {
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
