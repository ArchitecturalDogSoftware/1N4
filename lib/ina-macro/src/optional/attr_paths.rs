use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::{Ident, Path};

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
