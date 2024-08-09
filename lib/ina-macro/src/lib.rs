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

/// Implements the [`Stored`] derive macro.
mod stored;

/// Implements the [`Stored`](<ina_storage::stored::Stored>) trait for the deriving type.
///
/// # Examples
///
/// ```
/// #[derive(Stored)]
/// #[data_format(ina_storage::format::MessagePack)]
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
