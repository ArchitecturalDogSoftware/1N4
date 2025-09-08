use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Error, Expr, Field, Fields, FieldsNamed, FieldsUnnamed, Ident, Meta, MetaList, Result, Token, Type};

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

pub struct FieldsWithDefaults {
    /// The [`Ident`] of the struct with non-[optional] fields.
    ///
    /// [optional]: `Option`
    pub ident: Ident,
    /// The [`Ident`] of the struct with [optional] fields.
    ///
    /// [optional]: `Option`
    pub optional_ident: Ident,
    pub fields: Vec<FieldWithDefault>,
}

impl FieldsWithDefaults {
    pub fn generate_conversions(&self) -> TokenStream {
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

pub struct FieldWithDefault {
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
    pub fn new(field: &Field) -> Result<Self> {
        let option_attr_path = super::attr_paths::option();
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
                return Err(Error::new(
                    other.span(),
                    "expected annotation in the form of `#[option(default)]` or `#[option(default = EXPRESSION)]`",
                ));
            }
        };

        Ok(Self { ident, default })
    }
}

pub fn fields_to_optional(fields: Fields) -> Fields {
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
    let option_attr_path = super::attr_paths::option();
    if let Some(option_attr) = attrs.iter_mut().find(|attr| attr.path() == &option_attr_path) {
        option_attr.meta = Meta::List(MetaList {
            path: super::attr_paths::serde(),
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
