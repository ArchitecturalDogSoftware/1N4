// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024 Jaxydog
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

//! Provides procedural macros for 1N4.

use proc_macro::TokenStream;

/// Implements the [`AsTranslation`] derive macro.
mod as_translation;
/// Implements the [`macro@optional`] annotation macro.
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

/// Make the fields of a struct [optional], create a corresponding non-optional struct, and provide conversions.
///
/// Designed to make layered configurations with Clap easier.
///
/// # Usage
///
/// This supports four arguments, provided as a comma-separated list of `name = value` pairs:
///
/// - `keep_annotations`: A bracketed, comma-separated list of the [paths] of the [annotations] that are already present
///   on the optional struct that should be kept verbatim on the non-optional struct.
/// - `keep_field_annotations`: A bracketed, comma-separated list of the [paths] of the [annotations] that are already
///   present on the fields of the optional struct that should be kept verbatim on the fields of the non-optional
///   struct.
/// - `apply_derives`: A bracketed, comma-separated list of the [paths] of [`derive`] macros that should be applied to
///   the non-optional struct.
/// - `apply_annotations`: A braced, whitespace-separated list [annotations] that should be applied verbatim to the
///   non-optional struct.
///
/// Every field on the struct (unit and tuple structs are not supported)[^nontech] must be annotated with
/// `#[option(...)]` to provide default values. `#[option(default)]` will fill any [`None`] with [`Default::default`],
/// `#[option(default = EXPR)]` will fill any [`None`] with `EXPR`, and `#[option(flatten)]` will add Clap's
/// `#[command(flatten)]` annotation to the field and fill its defaults with `field.fill_defaults()`.
///
/// Two structs and six methods are modified or generated from this:
///
/// - The input struct (we will call `IDENT`) and its fields will have any [annotations] and [`derive`] macros not
///   specified with `keep_annotations`, `keep_field_annotations`, `apply_derives`, or `apply_annotations` stripped. Two
///   exceptions exist:
///     - Documentation comments are always kept.[^nontech]
///     - If `apply_derives` is provided, the provided [`derive`] annotation will be kept and filled with the given
///       macros. If it was provided, but no [`derive`] annotation is present, it will append a new one to fill.
/// - A new struct, called `OptionalIDENT` will be generated, with every annotation and [`derive`] macro retained.
///   [`macro@Default`] will prepended to the existing [`derive`] annotation, or added in a new [`derive`] annotation if
///   one isn't provided.
///   - For every field not annotated with `#[option(flatten)]`, the type will be wrapped in [`Option`] and annotated
///     with `#[serde(default, skip_serializing_if = "::std::option::Option::is_none")]`.
///   - For every field annotated with `#[option(flatten)]`, the type name will be prefixed with `Optional` (assuming
///     that it, too, had this macro applied to it) and annotated with `#[command(flatten)]` a similar Serde annotation.
///     For example, given some field of type `::my_other_crate::Settings`, it will be given the type
///     `::my_other_crate::OptionalSettings` and annotated with `#[serde(default, skip_serializing_if =
///     "<::my_other_crate::OptionalSettings>::is_all_none")]`.
/// - [`From<IDENT>`] will be implemented for `OptionalIDENT`, which just calls `.into()` on every field, which
///   effectively just wraps the value in [`Some`].
/// - On `OptionalIDENT`, five methods will be generated:
///   - `pub fn fill_defaults(self) -> IDENT`, which fills every field with its default value (generated based on the
///     `#[option(...)]` annotation of that field) to create an `IDENT`.
///   - `pub fn or(self, optb: Self) -> Self`, which calls [`Option::or`] (or this same generated method on fields
///     annotated with `#[option(flatten)]`) on each field. This is how you should layer multiple configuration sources
///     together.
///   - `pub fn xor(self, optb: Self) -> Self`, which calls [`Option::xor`] (or this same generated method on fields
///     annotated with `#[option(flatten)]`) on each field. It is hard to imagine a use case for this, so it will
///     probably be removed eventually.
///   - `pub fn is_all_some(self) -> bool`, which calls [`Option::is_some`] (or this same generated method on fields
///     annotated with `#[option(flatten)]`) on each field.
///   - `pub fn is_all_none(self) -> bool`, which calls [`Option::is_none`] (or this same generated method on fields
///     annotated with `#[option(flatten)]`) on each field.
///
/// # Notes
///
/// This only supports [outer annotations][`syn::AttrStyle::Outer`].[^nontech]
///
/// This does not support tuple structs or Clap subcommands.[^nontech] `#[command(flatten)]`, however, _is_ supported,
/// with the syntax `#[option(flatten)]`. In this case, it will (as previously mentioned) make the field "optional" by
/// renaming the identifier `IDENT` to `OptionalIDENT` and assume that `OptionalIDENT` has all the methods generated by
/// this macro.
///
/// If you define a type called `Option` in scope that's different from [`std::option::Option`], this will break.
/// Unfortunately, there's not a particularly good way around this, because Clap currently matches on `Option` (to
/// infer that an argument is optional) on a strictly textual basis, it doesn't attempt to infer from the actual type.
/// See [this comment](https://github.com/clap-rs/clap/issues/4636#issuecomment-1381969663) and
/// [this issue](https://github.com/clap-rs/clap/issues/4626).
///
/// # Examples
///
/// ```ignore
/// /// The application's command-line arguments.
/// #[non_exhaustive]
/// #[optional(
///     keep_annotations = [non_exhaustive],
///     apply_derives = [Clone, Debug, Hash, PartialEq, Eq],
/// )]
/// #[derive(Clone, Debug, Hash, PartialEq, Eq, Parser, Serialize)]
/// #[command(about, version)]
/// pub struct Arguments {
///     /// The bot's settings.
///     #[option(flatten)]
///     #[serde(rename = "client")]
///     pub bot_settings: Settings,
/// }
///
/// /// The bot's settings.
/// #[non_exhaustive]
/// #[optional(
///     keep_annotations = [non_exhaustive],
///     apply_derives = [Clone, Debug, Hash, PartialEq, Eq],
///     apply_annotations = {
///         #[expect(clippy::struct_excessive_bools, reason = "not relevant to CLI arguments")]
///     },
/// )]
/// #[derive(Clone, Debug, Hash, PartialEq, Eq, Args, Serialize, Deserialize)]
/// #[serde(rename_all = "kebab-case")]
/// #[group(id = "BotSettings")]
/// pub struct Settings {
///     /// The number of shards to spawn.
///     #[arg(short = 's', long = "shards")]
///     #[option(default)]
///     pub shards: Option<NonZeroU32>,
///     /// The interval at which to reshard in hours.
///     #[arg(short = 'r', long = "reshard-interval")]
///     #[option(default = self::default_reshard_interval())]
///     pub reshard_interval: NonZeroU64,
///
///     /// Disables all logger output.
///     #[arg(short = 'q', long = "quiet")]
///     #[option(default)]
///     pub quiet: bool,
/// }
///
/// /// Returns the default re-sharding interval.
/// fn default_reshard_interval() -> NonZeroU64 {
///     let Some(interval) = NonZeroU64::new(8) else { unreachable!("the default interval must be non-zero") };
///
///     interval
/// }
/// ```
///
/// [annotations]: `syn::Attribute`
/// [optional]: `Option`
/// [paths]: `syn::Path`
/// [^nontech]: It probably would not be that hard to support these, but we do not use any, so there is not a point in
///   adding the complexity. If you want to use them, let me (`@RemasteredArch`) know and I can add support.
#[proc_macro_attribute]
pub fn optional(attribute: TokenStream, item: TokenStream) -> TokenStream {
    match crate::optional::procedure(attribute, item) {
        Ok(token_stream) => token_stream,
        Err(error) => error.to_compile_error(),
    }
    .into()
}
