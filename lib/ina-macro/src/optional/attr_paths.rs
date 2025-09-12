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

use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::{Ident, Path};

/// Return a [`Path`] representing the `#[doc(...)]` macro.
#[must_use]
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
#[must_use]
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
#[must_use]
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
#[must_use]
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

/// Return a [`Path`] representing the `#[command(...)]` annotation.
// TO-DO: does this need to be replaced this with a qualified path?
#[must_use]
pub fn command() -> Path {
    Path {
        leading_colon: None,
        segments: {
            let mut punctuated = Punctuated::new();
            punctuated.push_value(Ident::new("command", Span::mixed_site()).into());
            punctuated
        },
    }
}
