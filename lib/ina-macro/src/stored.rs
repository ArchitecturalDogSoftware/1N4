// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024 Jaxydog
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

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{bracketed, parse_macro_input, Attribute, DeriveInput, Error, Ident, LitStr, Result, Token, Type};

/// The `data_format` attribute.
#[repr(transparent)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct StoredFormatAttribute {
    /// The format type.
    pub kind: Type,
}

impl StoredFormatAttribute {
    /// Parses the attribute.
    ///
    /// # Errors
    ///
    /// This function will return an error if the attribute fails to be parsed.
    pub fn parse(attribute: &Attribute) -> Result<Self> {
        attribute.parse_args_with(|input: ParseStream| Ok(Self { kind: input.parse()? }))
    }
}

/// The `data_path` attribute.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct StoredPathAttribute {
    /// The format literal.
    pub format: LitStr,
    /// The required types.
    pub arguments: Punctuated<Type, Token![,]>,
    /// The fields that create the path.
    pub fields: Punctuated<Ident, Token![,]>,
}

impl StoredPathAttribute {
    /// Parses the attribute.
    ///
    /// # Errors
    ///
    /// This function will return an error if the attribute fails to be parsed.
    pub fn parse(attribute: &Attribute) -> Result<Self> {
        mod kw {
            use syn::custom_keyword;

            custom_keyword!(fmt);
            custom_keyword!(args);
            custom_keyword!(from);
        }

        attribute.parse_args_with(|input: ParseStream| {
            input.parse::<kw::fmt>()?;
            input.parse::<Token![=]>()?;

            let format = input.parse::<LitStr>()?;

            input.parse::<Token![,]>()?;
            input.parse::<kw::args>()?;
            input.parse::<Token![=]>()?;

            let arguments_input;

            bracketed!(arguments_input in input);

            let arguments = arguments_input.parse_terminated(Type::parse, Token![,])?;

            input.parse::<Token![,]>()?;
            input.parse::<kw::from>()?;
            input.parse::<Token![=]>()?;

            let fields_input;

            bracketed!(fields_input in input);

            let fields = fields_input.parse_terminated(Ident::parse, Token![,])?;

            Ok(Self { format, arguments, fields })
        })
    }
}

/// Applies the procedural macro.
pub fn procedure(input: TokenStream) -> TokenStream {
    let DeriveInput { attrs: attributes, ident: identifier, generics, .. } = parse_macro_input!(input as DeriveInput);

    let Some(format_attribute) = attributes.iter().find(|a| a.path().is_ident("data_format")) else {
        return Error::new(identifier.span(), "missing `data_format` attribute").into_compile_error().into();
    };
    let format_type = match StoredFormatAttribute::parse(format_attribute) {
        Ok(StoredFormatAttribute { kind }) => kind,
        Err(error) => return error.into_compile_error().into(),
    };

    let Some(path_attribute) = attributes.iter().find(|a| a.path().is_ident("data_path")) else {
        return Error::new(identifier.span(), "missing `data_path` attribute").into_compile_error().into();
    };
    let (path_format, path_arguments, path_fields) = match StoredPathAttribute::parse(path_attribute) {
        Ok(StoredPathAttribute { format, arguments, fields }) => (format, arguments, fields),
        Err(error) => return error.into_compile_error().into(),
    };
    let path_arguments = path_arguments.iter();
    let path_fields = path_fields.iter();

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let path_format_arguments = (0 .. path_arguments.len()).map(|n| format_ident!("_{n}")).collect::<Box<[_]>>();

    quote! {
        impl #impl_generics ::ina_storage::stored::Stored for #identifier #type_generics
        #where_clause
        {
            type PathArguments = (#(#path_arguments),*);

            #[inline]
            fn data_format() -> impl ::ina_storage::format::DataFormat + ::std::marker::Send {
                <#format_type as ::std::default::Default>::default()
            }

            #[inline]
            fn data_path_for(
                (#(#path_format_arguments),*): <Self as ::ina_storage::stored::Stored>::PathArguments
            ) -> impl ::std::convert::AsRef<::std::path::Path> + ::std::marker::Send
            {
                ::std::format!(#path_format, #(#path_format_arguments),*)
            }

            #[inline]
            fn data_path(&self) -> impl ::std::convert::AsRef<::std::path::Path> + ::std::marker::Send {
                ::std::format!(#path_format, #(self.#path_fields),*)
            }
        }
    }
    .into()
}
