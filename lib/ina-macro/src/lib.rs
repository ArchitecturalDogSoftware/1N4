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
/// Implements the [`Stored`] derive macro.
mod stored;

/// Implements the [`AsTranslation`](<ina::utility::traits::convert::AsTranslation>) trait for the deriving type.
///
/// # Examples
///
/// A struct that represents a translatable input field.
///
/// ```ignore
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
/// ```ignore
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
/// ```ignore
/// #[derive(AsTranslation)]
/// #[localizer_category(category::UI)]
/// pub enum Type {
///     #[localizer_key("string")]
///     String,
///     #[localizer_key("number")]
///     Number,
/// }
///
/// impl Display for Type {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.localizer_key())
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

/// Implements the [`Stored`](<ina_storage::stored::Stored>) trait for the deriving type.
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
