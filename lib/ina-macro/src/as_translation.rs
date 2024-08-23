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
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    bracketed, parse_macro_input, Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Expr, Fields, Generics,
    Ident, LitStr, Result, Token, Variant,
};

/// The `localizer_category` attribute.
pub struct LocalizerCategoryAttribute {
    /// The category string.
    pub category: Expr,
}

impl LocalizerCategoryAttribute {
    /// Parses the attribute.
    ///
    /// # Errors
    ///
    /// This function will return an error if the attribute fails to be parsed.
    pub fn parse(attribute: &Attribute) -> Result<Self> {
        attribute.parse_args_with(|input: ParseStream| Ok(Self { category: input.parse()? }))
    }
}

pub enum LocalizerKeyAttribute {
    /// The literal key string.
    Literal(LitStr),
    /// A key based off of the given field.
    Field(Ident),
    /// A key based off of the given fields.
    Format(LitStr, Punctuated<Ident, Token![,]>),
}

impl LocalizerKeyAttribute {
    /// Parses the attribute.
    ///
    /// # Errors
    ///
    /// This function will return an error if the attribute fails to be parsed.
    pub fn parse(attribute: &Attribute) -> Result<Self> {
        mod kw {
            use syn::custom_keyword;

            custom_keyword!(fmt);
            custom_keyword!(from);
        }

        attribute.parse_args_with(|input: ParseStream| {
            if input.peek(kw::fmt) {
                input.parse::<kw::fmt>()?;
                input.parse::<Token![=]>()?;

                let fmt = input.parse::<LitStr>()?;

                input.parse::<Token![,]>()?;
                input.parse::<kw::from>()?;
                input.parse::<Token![=]>()?;

                let fields_input;

                bracketed!(fields_input in input);

                let from = fields_input.parse_terminated(Ident::parse, Token![,])?;

                Ok(Self::Format(fmt, from))
            } else if input.peek(kw::from) {
                input.parse::<kw::from>()?;
                input.parse::<Token![=]>()?;

                Ok(Self::Field(input.parse()?))
            } else {
                Ok(Self::Literal(input.parse()?))
            }
        })
    }
}

/// Applies the procedural macro.
///
/// ```
/// #[derive(AsTranslation)]
/// #[localizer_category(crate::utility::category::UI)]
/// pub enum DataType {
///     #[localizer_key("boolean")]
///     Boolean,
///     #[localizer_key("integer")]
///     Integer,
///     #[localizer_key("string")]
///     String,
/// }
/// ```
pub fn procedure(input: TokenStream) -> TokenStream {
    let DeriveInput { attrs, ident, generics, data, .. } = parse_macro_input!(input as DeriveInput);

    match data {
        Data::Struct(data) => self::procedure_struct(&attrs, &ident, &generics, data),
        Data::Enum(data) => self::procedure_enum(&attrs, &ident, &generics, data),
        Data::Union(_) => Error::new(ident.span(), "union types are not supported").into_compile_error().into(),
    }
}

/// Applies the procedural macro to a struct.
pub fn procedure_struct(
    attributes: &[Attribute],
    identifier: &Ident,
    generics: &Generics,
    DataStruct { fields, .. }: DataStruct,
) -> TokenStream {
    let category = match self::parse_attribute(
        attributes.iter(),
        "localizer_category",
        identifier.span(),
        LocalizerCategoryAttribute::parse,
    ) {
        Ok(LocalizerCategoryAttribute { category }) => category,
        Err(error) => return error.into_compile_error().into(),
    };

    let key = match self::parse_attribute(
        attributes.iter(),
        "localizer_key",
        identifier.span(),
        LocalizerKeyAttribute::parse,
    ) {
        Ok(LocalizerKeyAttribute::Literal(literal)) => quote! { #literal },
        Ok(LocalizerKeyAttribute::Field(field)) => quote! { ::std::format!("{}", &self.#field) },
        Ok(LocalizerKeyAttribute::Format(fmt, from)) => match fields {
            Fields::Unnamed(_) => {
                let bind = from.iter();
                let from = from.iter();

                quote! {
                    let Self(#(#bind,)* ..) = self;

                    ::std::format!(#fmt, #(&#from),*)
                }
            }
            Fields::Named(_) => {
                let from = from.into_iter();

                quote! { ::std::format!(#fmt, #(&self.#from),*) }
            }
            Fields::Unit if from.is_empty() => quote! { ::std::format!(#fmt) },
            Fields::Unit => {
                return Error::new(identifier.span(), "formatted keys are not valid for unit structs")
                    .into_compile_error()
                    .into();
            }
        },
        Err(error) => return error.into_compile_error().into(),
    };

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        #[automatically_derived]
        impl #impl_generics crate::utility::traits::convert::AsTranslation for #identifier #type_generics
        #where_clause
        {
            type Error = ::ina_localization::Error<(
                ::std::option::Option<::std::primitive::usize>,
                (
                    ::std::sync::Arc<::tokio::sync::RwLock<::ina_localization::Localizer>>,
                    ::ina_localization::thread::Request,
                ),
            )>;

            fn localizer_category(&self) -> impl ::std::convert::Into<::std::boxed::Box<::std::primitive::str>> {
                #category
            }

            fn localizer_key(&self) -> impl ::std::convert::Into<::std::boxed::Box<::std::primitive::str>> {
                #key
            }
        }
    }
    .into()
}

/// Applies the procedural macro to an enum.
pub fn procedure_enum(
    attributes: &[Attribute],
    identifier: &Ident,
    generics: &Generics,
    DataEnum { variants, .. }: DataEnum,
) -> TokenStream {
    let category = match self::parse_attribute(
        attributes.iter(),
        "localizer_category",
        identifier.span(),
        LocalizerCategoryAttribute::parse,
    ) {
        Ok(LocalizerCategoryAttribute { category }) => category,
        Err(error) => return error.into_compile_error().into(),
    };

    let mut variants_to_keys = Vec::with_capacity(variants.len());

    for Variant { attrs, ident, fields, .. } in variants {
        let (key, retained_fields) =
            match self::parse_attribute(attrs.iter(), "localizer_key", ident.span(), LocalizerKeyAttribute::parse) {
                Ok(LocalizerKeyAttribute::Literal(literal)) => (quote! { #literal }, vec![]),
                Ok(LocalizerKeyAttribute::Field(field)) => (quote! { &#field }, vec![field]),
                Ok(LocalizerKeyAttribute::Format(fmt, from)) => {
                    let fields = from.iter();

                    (quote! { ::std::format_args!(#fmt, #(&#fields),*) }, from.into_iter().collect())
                }
                Err(error) => return error.into_compile_error().into(),
            };

        let fields = match fields {
            Fields::Unit => TokenStream::new().into(),
            Fields::Named(_) => quote! { { #(#retained_fields,)* .. } },
            Fields::Unnamed(_) => quote! { (#(#retained_fields,)* ..) },
        };

        variants_to_keys.push(quote! { Self::#ident #fields => ::std::format!("{}", #key), });
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        #[automatically_derived]
        impl #impl_generics crate::utility::traits::convert::AsTranslation for #identifier #type_generics
        #where_clause
        {
            type Error = ::ina_localizing::Error;

            fn localizer_category(&self) -> impl ::std::convert::Into<::std::boxed::Box<::std::primitive::str>> {
                #category
            }

            fn localizer_key(&self) -> impl ::std::convert::Into<::std::boxed::Box<::std::primitive::str>> {
                match self {
                    #(#variants_to_keys)*
                }
            }
        }
    }
    .into()
}

/// Parses an attribute from the given list.
///
/// # Errors
///
/// This function will return an error if the attribute could not be parsed.
fn parse_attribute<'a, A>(
    mut attributes: impl Iterator<Item = &'a Attribute>,
    name: &str,
    span: Span,
    parse: impl FnOnce(&Attribute) -> Result<A>,
) -> Result<A> {
    #[inline]
    fn error(name: &str, span: Span) -> Error {
        Error::new(span, format_args!("missing `{name}` attribute, which is required for this derive"))
    }

    attributes.find(|a| a.path().is_ident(name)).ok_or_else(|| error(name, span)).and_then(parse)
}
