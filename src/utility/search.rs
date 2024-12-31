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

/// Determines how strict a contains search is.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strictness {
    /// All parts of the string must be found within the base string.
    Loose {
        /// Whether to ignore character casing.
        ignore_casing: bool,
    },
    /// String must match fairly well, ignoring non-alphanumeric characters.
    Firm {
        /// Whether to ignore character casing.
        ignore_casing: bool,
    },
    /// String must nearly match exactly.
    Strict {
        /// Whether to ignore character casing.
        ignore_casing: bool,
    },
}

impl Strictness {
    /// Returns `true` if the strictness is [`Loose`].
    ///
    /// [`Loose`]: Strictness::Loose
    #[must_use]
    pub const fn is_loose(&self) -> bool {
        matches!(self, Self::Loose { .. })
    }

    /// Returns `true` if the strictness is [`Firm`].
    ///
    /// [`Firm`]: Strictness::Firm
    #[must_use]
    pub const fn is_firm(&self) -> bool {
        matches!(self, Self::Firm { .. })
    }

    /// Returns `true` if the strictness is [`Strict`].
    ///
    /// [`Strict`]: Strictness::Strict
    #[must_use]
    pub const fn is_strict(&self) -> bool {
        matches!(self, Self::Strict { .. })
    }

    /// Returns whether this strictness level should ignore character casing.
    #[must_use]
    pub const fn ignore_casing(self) -> bool {
        match self {
            Self::Loose { ignore_casing } | Self::Firm { ignore_casing } | Self::Strict { ignore_casing } => {
                ignore_casing
            }
        }
    }
}

/// Returns whether the given pattern is contained within the provided string.
///
/// The strictness of the search is controlled by the [`Strictness`] argument.
pub fn fuzzy_contains(strictness: Strictness, string: impl AsRef<str>, pattern: impl AsRef<str>) -> bool {
    let mut string = string.as_ref().to_owned();
    let mut pattern = pattern.as_ref().to_owned();

    if strictness.ignore_casing() {
        string = string.to_lowercase();
        pattern = pattern.to_lowercase();
    }

    if strictness.is_loose() {
        string.retain(char::is_alphanumeric);
        pattern.retain(|c| c.is_alphanumeric() || c.is_whitespace());

        return pattern.trim().split(char::is_whitespace).all(|s| string.contains(s));
    }

    if strictness.is_firm() {
        string.retain(char::is_alphanumeric);
        pattern.retain(char::is_alphanumeric);
    }

    string.contains(&pattern)
}
