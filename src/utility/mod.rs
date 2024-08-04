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

/// Provides getters for client secrets.
pub mod secret;
/// Provides commonly-used trait definitions and blanket implementations.
pub mod traits;
/// Provides various useful types and builders.
pub mod types;

/// The base Discord CDN URL.
pub const DISCORD_CDN_URL: &str = "https://cdn.discordapp.com";
/// The base twemoji CDN URL.
pub const TWEMOJI_CDN_URL: &str = "https://raw.githubusercontent.com/discord/twemoji/main/assets/72x72";

macro_rules! define_categories {
    ($($name:ident => $value:literal;)*) => {
        /// Localizer category constants.
        #[allow(missing_docs)]
        pub mod category {
            pub const LIST: &[&str] = &[$(self::$name),*];

            $(pub const $name: &str = $value;)*
        }
    };
}

define_categories! {
    COMMAND => "command";
    COMMAND_OPTION => "command-option";
    COMMAND_CHOICE => "command-choice";

    UI => "ui";
    UI_BUTTON => "ui-button";
    UI_SELECT => "ui-select";
    UI_INPUT => "ui-input";
}

/// Color constants.
pub mod color {
    /// The bot's branding color (A).
    pub const BRANDING_A: u32 = 0x2C_8F_E5;
    /// The bot's branding color (B).
    pub const BRANDING_B: u32 = 0xE5_82_2C;

    /// The bot's backdrop color (A).
    pub const BACKDROP_A: u32 = 0x1C_4A_72;
    /// The bot's backdrop color (B).
    pub const BACKDROP_B: u32 = 0x72_44_1C;

    /// The bot's success color.
    pub const SUCCESS: u32 = 0x45_E0_51;
    /// The bot's failure color.
    pub const FAILURE: u32 = 0xDC_3F_31;
}

/// Determines how strict a contains search is.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strictness {
    /// All parts of the string must be found within the base string.
    Loose(bool),
    /// String must match fairly well, ignoring non-alphanumeric characters.
    Firm(bool),
    /// String must nearly match exactly.
    Strict(bool),
}

/// Returns whether `find` is roughly contained within `base`.
pub fn fuzzy_contains(strictness: Strictness, base: impl AsRef<str>, find: impl AsRef<str>) -> bool {
    match strictness {
        Strictness::Loose(ignore_casing) => {
            let mut base = base.as_ref().replace(|c: char| !c.is_alphanumeric(), "");
            let mut find = find.as_ref().replace(|c: char| !(c.is_alphanumeric() || c.is_whitespace()), "");

            if ignore_casing {
                base = base.to_lowercase();
                find = find.to_lowercase();
            }

            find.trim().split(char::is_whitespace).all(|s| base.contains(s))
        }
        Strictness::Firm(ignore_casing) => {
            let base = base.as_ref().replace(|c: char| !c.is_alphanumeric(), "");
            let find = find.as_ref().replace(|c: char| !c.is_alphanumeric(), "");

            if ignore_casing { base.to_lowercase().contains(&find.to_lowercase()) } else { base.contains(&find) }
        }
        Strictness::Strict(ignore_casing) => {
            let base = base.as_ref();
            let find = find.as_ref();

            if ignore_casing { base.to_lowercase().contains(&find.to_lowercase()) } else { base.contains(find) }
        }
    }
}
