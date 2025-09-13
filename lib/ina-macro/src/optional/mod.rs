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

use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DataStruct, DeriveInput, Error};

/// Parse item-level annotation arguments.
mod arguments;
/// Hardcoded [paths][`syn::Path`] representing various annotations.
mod attr_paths;
/// Parse fields and their annotation.
mod fields;

/// Prepends [`macro@Default`] to the first existing [`derive`] [`Attribute`], or appends a new [`derive`] [`Attribute`]
/// for just [`macro@Default`].
fn add_derive_default(attributes: &mut Vec<Attribute>) -> syn::Result<()> {
    let derive_annotation_path = &self::attr_paths::derive();

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

/// Applies [the procedural macro][`macro@crate::optional`].
pub fn procedure(
    attribute_args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let arguments: self::arguments::OptionalArguments = syn::parse(attribute_args)?;

    let DeriveInput { attrs: input_attrs, ident: input_ident, generics, vis, data } = syn::parse(item)?;
    // Ignore the semicolon because it should only ever appear for unit and tuple structs, which we don't support.
    let Data::Struct(DataStruct { struct_token, fields: input_fields, semi_token: _ }) = data else {
        return Err(Error::new(arguments.span(), "`optional` only supports structs"));
    };

    let optional_ident = quote::format_ident!("Optional{input_ident}");
    let ident = input_ident;

    let mut fields_with_defaults = self::fields::FieldsWithDefaults {
        ident: ident.clone(),
        optional_ident: optional_ident.clone(),
        fields: Vec::with_capacity(input_fields.len()),
    };

    let optional_fields = self::fields::fields_to_optional(input_fields.clone())?;
    let mut fields = input_fields;

    for field in &mut fields {
        fields_with_defaults.fields.push(self::fields::FieldWithDefault::new(field)?);

        arguments.retain_only_kept_field_attrs(field);
    }

    let conversions = fields_with_defaults.generate_conversions();

    let attrs = arguments.only_kept_and_applied_attrs(&input_attrs)?;
    let mut optional_attrs = input_attrs;
    self::add_derive_default(&mut optional_attrs)?;

    Ok(quote! {
        #( #optional_attrs )*
        #vis #struct_token #optional_ident #generics
        #optional_fields

        #( #attrs )*
        #vis #struct_token #ident #generics
        #fields

        #conversions
    })
}
