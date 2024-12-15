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

/// Provides utilities for searching strings.
pub mod search;
/// Provides functions for retrieving client secrets.
pub mod secret;

/// The base Discord CDN URL.
pub const DISCORD_CDN_URL: &str = "https://cdn.discordapp.com";
/// The base Twemoji CDN URL.
pub const TWEMOJI_CDN_URL: &str = "https://raw.githubusercontent.com/discord/twemoji/main/assets/72x72";

crate::define_categories! {
    COMMAND => "command";
    COMMAND_OPTION => "command-option";
    COMMAND_CHOICE => "command-choice";

    UNIT => "unit";

    UI => "ui";
    UI_BUTTON => "ui-button";
    UI_SELECT => "ui-select";
    UI_INPUT => "ui-input";
}

/// Color constants.
pub mod color {
    use super::types::color::Color;

    /// The bot's branding color (A).
    pub const BRANDING_A: Color = Color::u32(0x2C_8F_E5);
    /// The bot's branding color (B).
    pub const BRANDING_B: Color = Color::u32(0xE5_82_2C);

    /// The bot's backdrop color (A).
    pub const BACKDROP_A: Color = Color::u32(0x1C_4A_72);
    /// The bot's backdrop color (B).
    pub const BACKDROP_B: Color = Color::u32(0x72_44_1C);

    /// The bot's success color.
    pub const SUCCESS: Color = Color::u32(0x45_E0_51);
    /// The bot's failure color.
    pub const FAILURE: Color = Color::u32(0xDC_3F_31);
}

/// Provides commonly-used trait definitions and blanket implementations.
pub mod traits {
    /// Type conversion traits.
    pub mod convert;
    /// Type extension traits.
    pub mod extension;
}

/// Provides various useful types and builders.
pub mod types {
    /// A reference to an existing message.
    pub mod anchor;
    /// Provides various builders for model types.
    pub mod builder;
    /// Provides a definition for colors.
    pub mod color;
    /// A type that defines custom identifiers.
    pub mod id;
    /// A type that defines modal data.
    pub mod modal;
}

/// Defines localization category constants within their own 'category' module.
///
/// # Examples
///
/// ```
/// define_categories! {
///     TEXT => "text";
///     WORDS => "words";
///     OTHER => "other";
///     THINGS => "things";
/// }
///
/// localize!(async category::TEXT, "some_key").await?;
///
/// for category in category::LIST {
///     info!(async "categories include '{category}'").await?;
/// }
/// ```
#[macro_export]
macro_rules! define_categories {
    ($($name:ident => $value:literal;)*) => {
        /// Localizer category constants.
        #[expect(clippy::allow_attributes, reason = "false-positive relating to macro generation")]
        #[allow(missing_docs, reason = "the generated variable names should be self-describing")]
        pub mod category {
            pub const LIST: &[&str] = &[$(self::$name),*];

            $(pub const $name: &str = $value;)*
        }
    };
}
