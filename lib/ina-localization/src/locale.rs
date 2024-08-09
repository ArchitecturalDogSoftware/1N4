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

use std::fmt::Display;
use std::str::FromStr;

use clap::builder::{TypedValueParser, ValueParserFactory};
use clap::{Arg, Command};
use serde::de::{Unexpected, Visitor};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Represents a locale's region code.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum LocaleTerritory {
    /// A numeric code.
    Numeric(u16),
    /// An alphabetic code with two characters.
    Alpha2([char; 2]),
}

impl Display for LocaleTerritory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(n) => write!(f, "{n}"),
            Self::Alpha2([a, b]) => write!(f, "{a}{b}"),
        }
    }
}

impl FromStr for LocaleTerritory {
    type Err = Error;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        #[inline]
        pub fn next_char(c: &mut impl Iterator<Item = char>, p: impl FnOnce(&char) -> bool) -> Result<char> {
            let Some(character) = c.next() else { return Err(Error::MissingCharacter) };

            p(&character).then_some(character).ok_or(Error::InvalidCharacter(character))
        }

        if value.chars().all(char::is_numeric) {
            let Ok(number) = value.parse() else { return Err(Error::InvalidLocale(value.into())) };

            return Ok(Self::Numeric(number));
        }
        if value.chars().count() == 2 {
            let mut chars = value.chars().take(2);

            let territory_1 = next_char(&mut chars, char::is_ascii_alphabetic)?;
            let territory_2 = next_char(&mut chars, char::is_ascii_alphabetic)?;

            return Ok(Self::Alpha2([territory_1.to_ascii_uppercase(), territory_2.to_ascii_uppercase()]));
        }

        Err(Error::InvalidLocale(value.into()))
    }
}

/// A locale used within the localizer.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Locale([char; 2], Option<LocaleTerritory>);

impl Locale {
    /// Creates a new [`Locale`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the given locale contains a non-ascii-alphabetic character.
    pub fn new(language: [char; 2], mut territory: Option<LocaleTerritory>) -> Result<Self> {
        if !language.iter().all(char::is_ascii_alphabetic) {
            let territory = territory.map(|t| format!("-{t}")).unwrap_or_default();
            let locale = format!("{}{}{territory}", language[0], language[1]);

            return Err(Error::InvalidLocale(locale.into_boxed_str()));
        }
        if let Some(LocaleTerritory::Alpha2(ref mut chars)) = territory {
            if !chars.iter().all(char::is_ascii_alphabetic) {
                let territory = territory.map(|t| format!("-{t}")).unwrap_or_default();
                let locale = format!("{}{}{territory}", language[0], language[1]);

                return Err(Error::InvalidLocale(locale.into_boxed_str()));
            }

            chars.map(|c| c.to_ascii_uppercase());
        }

        Ok(Self(language.map(|c| c.to_ascii_lowercase()), territory))
    }

    /// Returns the language code of this [`Locale`].
    #[must_use]
    pub fn language(&self) -> String {
        let Self([l1, l2], _) = self;

        format!("{l1}{l2}")
    }

    /// Returns the territory code of this [`Locale`].
    #[must_use]
    pub fn territory(&self) -> Option<String> {
        self.1.map(|territory| format!("{territory}"))
    }
}

impl TryFrom<&str> for Locale {
    type Error = Error;

    #[inline]
    fn try_from(value: &str) -> Result<Self> {
        value.parse()
    }
}

impl TryFrom<[char; 4]> for Locale {
    type Error = Error;

    #[inline]
    fn try_from([l1, l2, t1, t2]: [char; 4]) -> Result<Self> {
        Self::new([l1, l2], Some(LocaleTerritory::Alpha2([t1, t2])))
    }
}

impl From<Locale> for String {
    #[inline]
    fn from(value: Locale) -> Self {
        value.to_string()
    }
}

impl Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.language())?;

        if let Some(territory) = self.territory() {
            write!(f, "-{territory}")?;
        }

        Ok(())
    }
}

impl FromStr for Locale {
    type Err = Error;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        #[inline]
        pub fn next_char(c: &mut impl Iterator<Item = char>, p: impl FnOnce(&char) -> bool) -> Result<char> {
            let Some(character) = c.next() else { return Err(Error::MissingCharacter) };

            p(&character).then_some(character).ok_or(Error::InvalidCharacter(character))
        }

        let mut chars = value.chars();

        let language_1 = next_char(&mut chars, char::is_ascii_alphabetic)?;
        let language_2 = next_char(&mut chars, char::is_ascii_alphabetic)?;

        let territory: Option<Result<_>> = next_char(&mut chars, |c| c == &'-').ok().map(|_| {
            let remaining = chars.collect::<Box<str>>();

            remaining.parse::<LocaleTerritory>()
        });

        Self::new([language_1, language_2], territory.transpose()?)
    }
}

impl ValueParserFactory for Locale {
    type Parser = LocaleParser;

    fn value_parser() -> Self::Parser {
        LocaleParser
    }
}

impl Serialize for Locale {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Locale {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct LocaleVisitor;

        impl Visitor<'_> for LocaleVisitor {
            type Value = Locale;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a valid locale string")
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(LocaleVisitor)
    }
}

/// Parses a locale from a command-line argument.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocaleParser;

impl TypedValueParser for LocaleParser {
    type Value = Locale;

    fn parse_ref(
        &self,
        cmd: &Command,
        arg: Option<&Arg>,
        value: &std::ffi::OsStr,
    ) -> std::result::Result<Self::Value, clap::Error> {
        let inner = clap::value_parser!(Box<str>);
        let value = inner.parse_ref(cmd, arg, value)?;

        if let Ok(locale) = value.parse() {
            return Ok(locale);
        }

        Err(clap::Error::new(clap::error::ErrorKind::ValueValidation))
    }
}
