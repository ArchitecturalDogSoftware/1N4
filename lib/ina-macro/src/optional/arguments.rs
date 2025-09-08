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

use proc_macro2::{Delimiter, Group, Span, TokenTree};
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, ExprArray, Ident, Path, Result, Token, braced};

/// A flexible companion to [`Parse`] for types that can be parsed from [token streams].
///
/// [token streams]: `proc_macro2::TokenStream`
trait FromStream: Sized {
    /// Parse a [`Self`] from a stream. Does not have to consume the stream fully.
    ///
    /// "Stream," in this case, referring to an iterator over [`TokenTree`]s, which can be produced
    /// by a [`proc_macro2::TokenStream`].
    fn from_stream(input: &mut impl Iterator<Item = TokenTree>, span: Span) -> Result<Self>;
}

/// A comma-separated list of values, which may or may not end with a comma.
///
/// Similar to [`syn::punctuated::Punctuated`].
#[derive(Debug)]
struct List<T> {
    /// The actually list of values and the commas that follow them. Only the last comma may be
    /// [`None`].
    pairs: Vec<(T, Option<syn::token::Comma>)>,
}

impl<T: FromStream> TryFrom<proc_macro2::TokenStream> for List<T> {
    type Error = Error;

    fn try_from(input: proc_macro2::TokenStream) -> std::result::Result<Self, Self::Error> {
        let span = input.span();
        let mut iter = input.into_iter().peekable();

        let mut pairs = Vec::new();

        while iter.peek().is_some() {
            let value = T::from_stream(&mut iter, span)?;
            let comma = match iter.next() {
                Some(TokenTree::Punct(punct)) if punct.as_char() == ',' => {
                    Some(syn::token::Comma { spans: [punct.span()] })
                }
                Some(other) => {
                    return Err(Error::new(other.span(), "unexpected token, expected a comma"));
                }
                None => None,
            };

            pairs.push((value, comma));

            if comma.is_none() {
                break;
            }
        }

        Ok(Self { pairs })
    }
}

impl<T: Parse> Parse for List<T> {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut pairs = Vec::new();

        while !input.is_empty() {
            let value = input.parse()?;
            let comma = if input.is_empty() { None } else { input.parse()? };

            pairs.push((value, comma));
        }

        Ok(Self { pairs })
    }
}

/// Represents either an [`Expr`] or a [`Group`]. Used as an intermediate parsing step for things
/// that are syntactically shaped like one or both of them.
///
/// The [`Parse`] implementation attempts to parse a [`Self::Expr`] before it attempts to parse a
/// [`Self::Group`].
#[derive(Debug)]
enum ExprOrGroup {
    Expr(Expr),
    Group(Group),
}

impl Parse for ExprOrGroup {
    fn parse(input: ParseStream) -> Result<Self> {
        // Hold a cursor at the start of the stream so that it can be parsed twice --- it seems as
        // if [`Expr::parse`] was advancing the stream even if it returned an error.
        let start = input.cursor();
        // Similarly, get the span _before_ attempting to parse the stream.
        let span = input.span();

        if let Ok(expr) = input.parse() {
            return Ok(Self::Expr(expr));
        }

        if let Ok(group) = syn::parse(start.token_stream().into()) {
            return Ok(Self::Group(group));
        }

        Err(Error::new(span, "could not parse as either an expression or as a group"))
    }
}

impl ToTokens for ExprOrGroup {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Expr(expr) => expr.to_tokens(tokens),
            Self::Group(group) => group.to_tokens(tokens),
        }
    }
}

/// Represents the tokens `IDENT = EXPR`, where `IDENT` is any [`Ident`] and `EXPR` is an [`ExprOrGroup`]. This is
/// essentially a more flexible version of [`syn::MetaNameValue`].
#[expect(dead_code, reason = "keeping dead fields in case a refactor needs them")]
struct ArbitraryNameValue {
    /// The left-hand side of the expression.
    ident: Ident,
    eq_token: Token![=],
    /// The right-hand side of the expression.
    value: ExprOrGroup,
    /// The span of the original invocation tokens, running all the way from the start of [`Self::ident`] to the end
    /// of [`Self::value`].
    span: Span,
}

impl FromStream for ArbitraryNameValue {
    fn from_stream(input: &mut impl Iterator<Item = TokenTree>, span: Span) -> Result<Self> {
        macro_rules! unexpected_end {
            ($maybe:expr) => {
                $maybe.ok_or_else(|| Error::new(span, "unexpected end of input"))?
            };
        }

        let ident = match unexpected_end!(input.next()) {
            TokenTree::Ident(ident) => ident,
            other => return Err(Error::new(other.span(), format!("expect an identifier, received {other}"))),
        };
        let eq_token = match unexpected_end!(input.next()) {
            TokenTree::Punct(punct) if punct.as_char() == '=' => syn::token::Eq { spans: [punct.span()] },
            other => {
                return Err(Error::new(other.span(), format!("expect an equal sign (=), received {other}")));
            }
        };
        let value = match unexpected_end!(input.next()) {
            TokenTree::Group(group) => ExprOrGroup::Group(group),
            other => {
                let expr: Expr = syn::parse(other.to_token_stream().into())?;
                ExprOrGroup::Expr(expr)
            }
        };
        let span = ident
            .span()
            .join(eq_token.span())
            .and_then(|span| span.join(value.span()))
            .ok_or_else(|| Error::new(span, "received a token stream that crosses between files"))?;

        Ok(Self { ident, eq_token, value, span })
    }
}

impl Parse for ArbitraryNameValue {
    fn parse(input: ParseStream) -> Result<Self> {
        let span = input.span();
        Ok(Self { ident: input.parse()?, eq_token: input.parse()?, value: input.parse()?, span })
    }
}

/// Represents a list of [outer] [annotations].
///
/// The [`Parse`] and [`TryFrom<ExprOrGroup>`] implementations of this type expect a braced list of
/// space-separated annotations, e.g., `{ #[allow(dead_code)] #[my_cool_proc_macro] }`.
///
/// [outer]: `syn::AttrStyle::Outer`
/// [annotations]: `Attribute`
#[derive(Debug)]
struct AttributeList {
    attributes: Vec<Attribute>,
}

impl TryFrom<ExprOrGroup> for AttributeList {
    type Error = Error;

    fn try_from(braced_list: ExprOrGroup) -> std::result::Result<Self, Self::Error> {
        let span = braced_list.span();
        let error_msg = || {
            Error::new(
                span,
                "expected a braced list of space-separated annotations, e.g., `{ #[allow(dead_code)] \
                 #[my_cool_proc_macro] }`",
            )
        };

        let ExprOrGroup::Group(group) = braced_list else {
            return Err(error_msg());
        };
        if group.delimiter() != Delimiter::Brace {
            return Err(error_msg());
        }

        syn::parse(group.to_token_stream().into()).map_err(|_| error_msg())
    }
}

impl Parse for AttributeList {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _ = braced!(content in input);

        Attribute::parse_outer(&content).map(|attributes| Self { attributes })
    }
}

/// Parse a bracketed list of comma-separated [`Path`]s into a [`Vec<Path>`]. A trailing comma is
/// optional.
fn parse_paths(bracketed_list: ExprOrGroup) -> Result<Vec<Path>> {
    Ok(match bracketed_list {
        ExprOrGroup::Expr(Expr::Array(ExprArray { elems, .. })) => elems
            .into_iter()
            .map(|expr| match expr {
                Expr::Path(path) => Ok(path.path),
                other => Err(Error::new(other.span(), "expected path")),
            })
            .collect::<Result<_>>()?,
        ExprOrGroup::Group(group) => {
            syn::parse::<List<Path>>(group.stream().into())?.pairs.into_iter().map(|(path, _)| path).collect()
        }
        ExprOrGroup::Expr(other) => {
            return Err(Error::new(other.span(), "expected comma-separated list of paths"));
        }
    })
}

/// Represents the arguments of the [`crate::optional`] procedural macro.
///
/// These are implemented though what is effectively a more flexible [`syn::Meta`] parser.
#[derive(Debug)]
pub struct OptionalArguments {
    pub keep_derives: Option<Vec<Path>>,
    pub keep_annotations: Vec<Path>,
    pub keep_field_annotations: Vec<Path>,
    pub apply_annotations: Option<Vec<Attribute>>,
    pub attr_span: Span,
}

impl Parse for OptionalArguments {
    fn parse(input: ParseStream) -> Result<Self> {
        // Get the [`Span`] of the input before we drain it into a token stream.
        let attr_span = input.span();

        // Using the obvious solution of [`ParseStream::parse::<ArbitraryNameValue>`] would also
        // pass the comma separating each [`ArbitraryNameValue`] in the list to their parser, which
        // would make [`ParseStream::parse`] mad because it expects the stream to be empty by the
        // end. Using a custom approach based on token streams did not have this issue.
        let arguments = List::<ArbitraryNameValue>::try_from(input.parse::<proc_macro2::TokenStream>()?)?;

        let mut keep_derives = None;
        let mut keep_annotations = None;
        let mut keep_field_annotations = None;
        let mut apply_annotations = None;

        for (ArbitraryNameValue { ident, value, .. }, _) in arguments.pairs {
            match ident.to_string().as_str() {
                "keep_derives" => keep_derives = Some(parse_paths(value)?),
                "keep_annotations" => keep_annotations = Some(parse_paths(value)?),
                "keep_field_annotations" => keep_field_annotations = Some(parse_paths(value)?),
                "apply_annotations" => {
                    apply_annotations = Some(AttributeList::try_from(value)?.attributes);
                }
                other => return Err(Error::new(ident.span(), format!("unknown argument: {other}"))),
            }
        }

        // Always maintain documentation comments on both structs.
        let mut keep_annotations: Vec<Path> = keep_annotations.unwrap_or_else(|| Vec::with_capacity(1));
        keep_annotations.push(super::attr_paths::doc());

        // Maintain the [`derive`] annotation on the non-optional struct if the consumer specified
        // the some number of the derives should be maintained.
        if keep_derives.is_some() {
            keep_annotations.push(super::attr_paths::derive());
        }

        // Always maintain documentation comments on both structs' fields.
        let mut keep_field_annotations = keep_field_annotations.unwrap_or_else(|| Vec::with_capacity(1));
        keep_field_annotations.push(super::attr_paths::doc());

        Ok(Self { keep_derives, keep_annotations, keep_field_annotations, apply_annotations, attr_span })
    }
}
