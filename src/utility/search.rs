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

/// Returns whether `find` is roughly contained within `base`.
pub fn fuzzy_contains(strictness: Strictness, base: impl AsRef<str>, find: impl AsRef<str>) -> bool {
    match strictness {
        Strictness::Loose { ignore_casing } => {
            let mut base = base.as_ref().replace(|c: char| !c.is_alphanumeric(), "");
            let mut find = find.as_ref().replace(|c: char| !(c.is_alphanumeric() || c.is_whitespace()), "");

            if ignore_casing {
                base = base.to_lowercase();
                find = find.to_lowercase();
            }

            find.trim().split(char::is_whitespace).all(|s| base.contains(s))
        }
        Strictness::Firm { ignore_casing } => {
            let base = base.as_ref().replace(|c: char| !c.is_alphanumeric(), "");
            let find = find.as_ref().replace(|c: char| !c.is_alphanumeric(), "");

            if ignore_casing { base.to_lowercase().contains(&find.to_lowercase()) } else { base.contains(&find) }
        }
        Strictness::Strict { ignore_casing } => {
            let base = base.as_ref();
            let find = find.as_ref();

            if ignore_casing { base.to_lowercase().contains(&find.to_lowercase()) } else { base.contains(find) }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::utility::search::Strictness;

    const HAYSTACK: &str = "AaAbBbCcC!@#123";

    fn search(strictness: Strictness, needle: &'static str, pass: bool) {
        assert_eq!(super::fuzzy_contains(strictness, HAYSTACK, needle), pass);
    }

    #[test]
    fn search_loose() {
        let sensitive = Strictness::Loose { ignore_casing: false };
        let insensitive = Strictness::Loose { ignore_casing: true };

        self::search(sensitive, "aaabbbccc123", false);
        self::search(insensitive, "aaabbbccc123", true);
        self::search(sensitive, "aaa bbb ccc 123", false);
        self::search(insensitive, "aaa bbb ccc 123", true);

        self::search(sensitive, "AAABBBCCC123", false);
        self::search(insensitive, "AAABBBCCC123", true);
        self::search(sensitive, "AAA BBB CCC 123", false);
        self::search(insensitive, "AAA BBB CCC 123", true);

        self::search(sensitive, "AaAbBbCcC123", true);
        self::search(insensitive, "AaAbBbCcC123", true);
        self::search(sensitive, "AaA bBb CcC 123", true);
        self::search(insensitive, "AaA bBb CcC 123", true);

        self::search(sensitive, "AaAbBbCcC!@#123", true);
        self::search(insensitive, "AaAbBbCcC!@#123", true);
        self::search(sensitive, "AaA bBb CcC !@# 123", true);
        self::search(insensitive, "AaA bBb CcC !@# 123", true);

        self::search(sensitive, "AaAbBbCcC123CcCbBbAaA", false);
        self::search(insensitive, "AaAbBbCcC123CcCbBbAaA", false);
        self::search(sensitive, "AaA bBb CcC 123 CcC bBb AaA", true);
        self::search(insensitive, "AaA bBb CcC 123 CcC bBb AaA", true);

        self::search(sensitive, "c?!?!?!?!??!?! 123...", true);
        self::search(insensitive, "c?!?!?!?!??!?! 123...", true);

        self::search(sensitive, "?", true);
        self::search(insensitive, "?", true);

        self::search(sensitive, "", true);
        self::search(insensitive, "", true);

        self::search(sensitive, "🦀", true);
        self::search(insensitive, "🦀", true);

        self::search(sensitive, "unrelated string", false);
        self::search(insensitive, "unrelated string", false);
    }

    #[test]
    fn search_firm() {
        let sensitive = Strictness::Firm { ignore_casing: false };
        let insensitive = Strictness::Firm { ignore_casing: true };

        self::search(sensitive, "aaabbbccc123", false);
        self::search(insensitive, "aaabbbccc123", true);
        self::search(sensitive, "aaa bbb ccc 123", false);
        self::search(insensitive, "aaa bbb ccc 123", true);

        self::search(sensitive, "AAABBBCCC123", false);
        self::search(insensitive, "AAABBBCCC123", true);
        self::search(sensitive, "AAA BBB CCC 123", false);
        self::search(insensitive, "AAA BBB CCC 123", true);

        self::search(sensitive, "AaAbBbCcC123", true);
        self::search(insensitive, "AaAbBbCcC123", true);
        self::search(sensitive, "AaA bBb CcC 123", true);
        self::search(insensitive, "AaA bBb CcC 123", true);

        self::search(sensitive, "AaAbBbCcC!@#123", true);
        self::search(insensitive, "AaAbBbCcC!@#123", true);
        self::search(sensitive, "AaA bBb CcC !@# 123", true);
        self::search(insensitive, "AaA bBb CcC !@# 123", true);

        self::search(sensitive, "AaAbBbCcC123CcCbBbAaA", false);
        self::search(insensitive, "AaAbBbCcC123CcCbBbAaA", false);
        self::search(sensitive, "AaA bBb CcC 123 CcC bBb AaA", false);
        self::search(insensitive, "AaA bBb CcC 123 CcC bBb AaA", false);

        self::search(sensitive, "c?!?!?!?!??!?! 123...", false);
        self::search(insensitive, "c?!?!?!?!??!?! 123...", true);

        self::search(sensitive, "?", true);
        self::search(insensitive, "?", true);

        self::search(sensitive, "", true);
        self::search(insensitive, "", true);

        self::search(sensitive, "🦀", true);
        self::search(insensitive, "🦀", true);

        self::search(sensitive, "unrelated string", false);
        self::search(insensitive, "unrelated string", false);
    }

    #[test]
    fn search_strict() {
        let sensitive = Strictness::Strict { ignore_casing: false };
        let insensitive = Strictness::Strict { ignore_casing: true };

        self::search(sensitive, "aaabbbccc123", false);
        self::search(insensitive, "aaabbbccc123", false);
        self::search(sensitive, "aaa bbb ccc 123", false);
        self::search(insensitive, "aaa bbb ccc 123", false);

        self::search(sensitive, "AAABBBCCC123", false);
        self::search(insensitive, "AAABBBCCC123", false);
        self::search(sensitive, "AAA BBB CCC 123", false);
        self::search(insensitive, "AAA BBB CCC 123", false);

        self::search(sensitive, "AaAbBbCcC123", false);
        self::search(insensitive, "AaAbBbCcC123", false);
        self::search(sensitive, "AaA bBb CcC 123", false);
        self::search(insensitive, "AaA bBb CcC 123", false);

        self::search(sensitive, "AaAbBbCcC!@#123", true);
        self::search(insensitive, "AaAbBbCcC!@#123", true);
        self::search(sensitive, "AaA bBb CcC !@# 123", false);
        self::search(insensitive, "AaA bBb CcC !@# 123", false);

        self::search(sensitive, "AaAbBbCcC123CcCbBbAaA", false);
        self::search(insensitive, "AaAbBbCcC123CcCbBbAaA", false);
        self::search(sensitive, "AaA bBb CcC 123 CcC bBb AaA", false);
        self::search(insensitive, "AaA bBb CcC 123 CcC bBb AaA", false);

        self::search(sensitive, "c?!?!?!?!??!?! 123...", false);
        self::search(insensitive, "c?!?!?!?!??!?! 123...", false);

        self::search(sensitive, "?", false);
        self::search(insensitive, "?", false);

        self::search(sensitive, "", true);
        self::search(insensitive, "", true);

        self::search(sensitive, "🦀", false);
        self::search(insensitive, "🦀", false);

        self::search(sensitive, "unrelated string", false);
        self::search(insensitive, "unrelated string", false);
    }
}
