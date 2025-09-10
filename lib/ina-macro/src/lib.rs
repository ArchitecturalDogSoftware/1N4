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

//! Provides procedural macros for 1N4.

use proc_macro::TokenStream;

/// Implements the [`AsTranslation`] derive macro.
mod as_translation;
/// Implements the [`optional`] annotation macro.
mod optional;
/// Implements the [`Stored`] derive macro.
mod stored;

/// Implements the `ina_localizing::AsTranslation` trait for the deriving type.
///
/// # Examples
///
/// A struct that represents a translatable input field.
///
/// ```
/// # use ina_macro::AsTranslation;
/// # use ina_localizing::AsTranslation;
/// # mod category { pub const UI: &str = ""; }
/// #[derive(AsTranslation)]
/// #[localizer_category(category::UI)]
/// #[localizer_key(from = title)]
/// pub struct Field<'s> {
///     title: &'s str,
///     content: String,
/// }
/// ```
///
/// An enum of values that can be translated.
///
/// ```
/// # use ina_macro::AsTranslation;
/// # use ina_localizing::AsTranslation;
/// # mod category { pub const UI: &str = ""; }
/// #[derive(AsTranslation)]
/// #[localizer_category(category::UI)]
/// pub enum DataType {
///     #[localizer_key("boolean")]
///     Boolean,
///     #[localizer_key("integer")]
///     Integer,
///     #[localizer_key("string")]
///     String,
/// }
/// ```
///
/// A struct with a complex key.
///
/// ```
/// # use ina_macro::AsTranslation;
/// # use ina_localizing::AsTranslation;
/// # mod category { pub const UI: &str = ""; }
/// #[derive(AsTranslation)]
/// #[localizer_category(category::UI)]
/// pub enum Type {
///     #[localizer_key("string")]
///     String,
///     #[localizer_key("number")]
///     Number,
/// }
///
/// impl std::fmt::Display for Type {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.localizer_key().into())
///     }
/// }
///
/// #[derive(AsTranslation)]
/// #[localizer_category(category::UI)]
/// #[localizer_key(fmt = "{}-{}", from = [kind, name])]
/// pub struct TypedField {
///     pub kind: Type,
///     pub name: String,
/// }
/// ```
#[proc_macro_derive(AsTranslation, attributes(localizer_key, localizer_category))]
pub fn as_translation(input: TokenStream) -> TokenStream {
    crate::as_translation::procedure(input)
}

/// Implements the `ina_storage::stored::Stored` trait for the deriving type.
///
/// # Examples
///
/// Simple derive:
///
/// ```
/// # use serde::{Deserialize, Serialize};
/// # use ina_macro::Stored;
/// # use ina_storage::format::{Compress, Messagepack};
/// #[derive(Serialize, Deserialize, Stored)]
/// #[data_format(Compress<Messagepack>)] // this will use `Compress::default()`
/// #[data_path(fmt = "dir/{}", args = [String], from = [name])]
/// struct DataStructure {
///     name: String,
///     value: u64,
/// }
/// ```
///
/// Derive with custom format creation method:
///
/// ```
/// # use serde::{Deserialize, Serialize};
/// # use ina_macro::Stored;
/// # use ina_storage::format::{Compress, Messagepack};
/// #[derive(Serialize, Deserialize, Stored)]
/// #[data_format(kind = Compress<Messagepack>, from = Compress::new_fast(Messagepack))]
/// #[data_path(fmt = "dir/{}", args = [String], from = [name])]
/// struct DataStructure {
///     name: String,
///     value: u64,
/// }
/// ```
#[proc_macro_derive(Stored, attributes(data_path, data_format))]
pub fn stored(input: TokenStream) -> TokenStream {
    crate::stored::procedure(input)
}

/// Make the fields of a struct [optional], create a corresponding non-optional struct, and provide
/// conversions.
///
/// This does not support tuple structs or Clap subcommands. `#[command(flatten)]`, however, _is_
/// supported, with the syntax `#[option(flatten)]`. In this case, it will treat make the field
/// "optional" by renaming the identifier `IDENT` to `OptionalIDENT`, e.g., the optional form of
/// `::my_other_crate::Settings` is assumed to be `::my_other_crate::OptionalSettings`, and there
/// is assumed to be a method `::my_other_crate::OptionalSettings::fill_defaults(&self) ->
/// ::my_other_crate::Settings`.
///
/// If you define a type called `Option` in scope that's different from [`std::option::Option`],
/// this will break. Unfortunately, there's not a particularly good way around this, because Clap
/// currently matches on `Option` (to infer that an argument is optional) on a strictly textual
/// basis, it doesn't attempt to infer from the actual type. See
/// [this comment](https://github.com/clap-rs/clap/issues/4636#issuecomment-1381969663) and
/// [this issue](https://github.com/clap-rs/clap/issues/4626).
///
/// [optional]: `Option`
#[proc_macro_attribute]
pub fn optional(attribute: TokenStream, item: TokenStream) -> TokenStream {
    crate::optional::procedure(attribute, item)
}
