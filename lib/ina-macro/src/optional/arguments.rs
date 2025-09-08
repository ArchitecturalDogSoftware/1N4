use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Spacing, Span, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Error, Expr, ExprArray, Field, Fields, FieldsNamed, FieldsUnnamed, Ident,
    Meta, MetaList, MetaNameValue, Path, Result, Token, Type, braced, bracketed, parse_macro_input,
};

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
        #[derive(Debug)]
        enum ExprOrGroup {
            Expr(Expr),
            Group(Group),
        }

        impl Parse for ExprOrGroup {
            fn parse(input: ParseStream) -> Result<Self> {
                // Hold a cursor at the start of the stream so that it can be parsed twice --- it
                // seems as if [`Expr::parse`] was advancing the stream even if it returned an
                // error.
                let start = input.cursor();

                if let Ok(expr) = dbg!(input.parse()) {
                    return Ok(Self::Expr(expr));
                }

                if let Ok(group) = dbg!(syn::parse(start.token_stream().into())) {
                    return Ok(Self::Group(group));
                }

                Err(Error::new(
                    input.span(),
                    format!("could not parse as either an expression or as a group: {}", start.token_stream(),),
                ))
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

        struct ArbitraryNameValue {
            ident: Ident,
            eq_token: Token![=],
            value: ExprOrGroup,
        }

        impl ArbitraryNameValue {
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

                Ok(Self { ident, eq_token, value })
            }
        }

        impl Parse for ArbitraryNameValue {
            fn parse(input: ParseStream) -> Result<Self> {
                Ok(Self { ident: input.parse()?, eq_token: input.parse()?, value: input.parse()? })
            }
        }

        struct NameValueList {
            pairs: Vec<(ArbitraryNameValue, Option<Token![,]>)>,
        }

        impl TryFrom<proc_macro2::TokenStream> for NameValueList {
            type Error = syn::Error;

            fn try_from(input: proc_macro2::TokenStream) -> Result<Self> {
                let span = input.span();
                let mut iter = input.into_iter().peekable();

                let mut pairs = Vec::new();

                while iter.peek().is_some() {
                    let name_value = ArbitraryNameValue::from_stream(&mut iter, span)?;
                    let comma = match iter.next() {
                        Some(TokenTree::Punct(punct)) if punct.as_char() == ',' => {
                            Some(syn::token::Comma { spans: [punct.span()] })
                        }
                        Some(other) => {
                            return Err(Error::new(other.span(), "unexpected token, expected a comma"));
                        }
                        None => None,
                    };

                    pairs.push((name_value, comma));

                    if comma.is_none() {
                        break;
                    }
                }

                Ok(Self { pairs })
                // syn::parse(input.into())
            }
        }

        impl Parse for NameValueList {
            fn parse(input: ParseStream) -> Result<Self> {
                let mut pairs = Vec::new();

                while !input.is_empty() {
                    let name_value = input.parse()?;
                    let comma = if input.is_empty() { None } else { input.parse()? };

                    pairs.push((name_value, comma));
                }

                Ok(Self { pairs })
            }
        }

        #[derive(Debug)]
        struct AttributeList {
            attributes: Vec<Attribute>,
        }

        impl Parse for AttributeList {
            fn parse(input: ParseStream) -> Result<Self> {
                let content;
                let _ = braced!(content in input);

                Attribute::parse_outer(&content).map(|attributes| Self { attributes })
            }
        }

        #[derive(Debug)]
        struct PathList {
            pairs: Vec<(Path, Option<Token![,]>)>,
        }

        impl Parse for PathList {
            fn parse(input: ParseStream) -> Result<Self> {
                let mut pairs = Vec::new();

                while !input.is_empty() {
                    let path = input.parse()?;
                    let comma = if input.is_empty() { None } else { input.parse()? };

                    pairs.push((path, comma));
                }

                Ok(Self { pairs })
            }
        }

        fn parse_paths(bracketed_list: ExprOrGroup) -> Result<Vec<Path>> {
            Ok(match bracketed_list {
                ExprOrGroup::Expr(Expr::Array(ExprArray { elems, .. })) => elems
                    .into_iter()
                    .map(|expr| match expr {
                        Expr::Path(path) => Ok(path.path),
                        other => Err(Error::new(other.span(), format!("expected path, received {other:#?}"))),
                    })
                    .collect::<Result<_>>()?,
                ExprOrGroup::Group(group) => {
                    syn::parse::<PathList>(group.stream().into())?.pairs.into_iter().map(|(path, _)| path).collect()
                }
                ExprOrGroup::Expr(other) => {
                    return Err(Error::new(
                        other.span(),
                        format!("expected comma-separated list of paths, received {other:#?}"),
                    ));
                }
            })
        }

        fn parse_annotations(braced_list: ExprOrGroup) -> Result<Vec<Attribute>> {
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

            syn::parse::<AttributeList>(group.to_token_stream().into())
                .map_err(|_| error_msg())
                .map(|AttributeList { attributes }| attributes)
        }

        let attr_span = input.span();

        let arguments = NameValueList::try_from(input.parse::<proc_macro2::TokenStream>()?)?;
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
                    apply_annotations = Some(parse_annotations(value)?);
                }
                other => return Err(Error::new(ident.span(), format!("unknown argument: {other}"))),
            }
        }

        let mut keep_annotations: Vec<Path> = keep_annotations.unwrap_or_else(|| Vec::with_capacity(1));
        keep_annotations.push(super::attr_paths::doc());

        if keep_derives.is_some() {
            keep_annotations.push(super::attr_paths::derive());
        }

        let mut keep_field_annotations = keep_field_annotations.unwrap_or_else(|| Vec::with_capacity(1));
        keep_field_annotations.push(super::attr_paths::doc());

        Ok(Self { keep_derives, keep_annotations, keep_field_annotations, apply_annotations, attr_span })
    }
}
