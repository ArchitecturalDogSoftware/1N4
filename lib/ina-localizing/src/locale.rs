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

use std::ffi::OsStr;
use std::fmt::Display;
use std::str::FromStr;

use clap::builder::{TypedValueParser, ValueParserFactory};
use clap::{Arg, Command};
use serde::de::{Unexpected, Visitor};
use serde::{Deserialize, Serialize};

/// An error that may be returned when using this library.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A character was invalid.
    #[error("invalid character; expecting {0}, found {1}")]
    InvalidCharacter(&'static str, char),
    /// A string was parsed with an invalid length.
    #[error("invalid string length: {0}")]
    InvalidLength(usize),
    /// A given language code was invalid.
    #[error("invalid language code: {0:?}")]
    InvalidLanguage([char; 2]),
    /// A given territory code was invalid.
    #[error("invalid territory code: '{0}'")]
    InvalidTerritory(Territory),
    /// A character was missing.
    #[error("missing character; expecting {0}")]
    MissingCharacter(&'static str),
    /// A numeric code failed to parse.
    #[error(transparent)]
    Parse(#[from] std::num::ParseIntError),
}

/// A regional linguistic locale.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Locale {
    /// The locale's language code.
    language: [char; 2],
    /// The locale's territory identifier.
    territory: Option<Territory>,
}

impl Default for Locale {
    fn default() -> Self {
        Self { language: ['e', 'n'], territory: Some(Territory::Alpha2(['U', 'S'])) }
    }
}

impl Locale {
    /// Creates a new [`Locale`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the given language or territory codes are invalid.
    pub fn new(language: [char; 2], territory: Option<Territory>) -> Result<Self, Error> {
        if !language.iter().all(char::is_ascii_lowercase) {
            return Err(Error::InvalidLanguage(language));
        }

        if territory.is_some_and(|t| match t {
            Territory::Alpha2(c) => !c.iter().all(char::is_ascii_uppercase),
            Territory::Alpha3(c) => !c.iter().all(char::is_ascii_uppercase),
            Territory::Numeric(_) => false,
        }) {
            return Err(Error::InvalidTerritory(territory.unwrap_or_else(|| unreachable!())));
        }

        Ok(Self { language, territory })
    }

    /// Returns the locale's language code.
    #[must_use]
    pub fn language(&self) -> Box<str> {
        self.language.iter().collect()
    }

    /// Returns the locale's territory code.
    #[must_use]
    pub fn territory(&self) -> Option<Box<str>> {
        self.territory.map(|t| t.to_string().into_boxed_str())
    }
}

impl TryFrom<&str> for Locale {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl FromStr for Locale {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.chars();
        let next_char = |_| self::match_next_char(&mut iter, char::is_ascii_lowercase, "a lowercase ascii character");

        let language = std::array::try_from_fn(next_char)?;
        let territory = self::match_next_char(&mut iter, |c| c == &'-', "a hyphen character")
            .ok()
            .map(|_| iter.collect::<Box<str>>().parse())
            .transpose()?;

        Self::new(language, territory)
    }
}

impl From<Locale> for Box<str> {
    fn from(value: Locale) -> Self {
        value.to_string().into_boxed_str()
    }
}

impl From<Locale> for String {
    fn from(value: Locale) -> Self {
        value.to_string()
    }
}

impl Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.language().fmt(f)?;

        if let Some(territory) = self.territory {
            write!(f, "-{territory}")?;
        }

        Ok(())
    }
}

impl ValueParserFactory for Locale {
    type Parser = LocaleValueParser;

    fn value_parser() -> Self::Parser {
        LocaleValueParser
    }
}

impl Serialize for Locale {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
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

/// A locale's territory identifier.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Territory {
    /// A two-letter alphabetic territory code.
    Alpha2([char; 2]),
    /// A three-letter alphabetic territory code.
    Alpha3([char; 3]),
    /// A numeric territory code.
    Numeric(u32),
}

impl FromStr for Territory {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.chars().all(char::is_numeric) {
            return Ok(s.parse().map(Self::Numeric)?);
        }

        let mut iter = s.chars();
        let next_char = |_| self::match_next_char(&mut iter, char::is_ascii_uppercase, "an uppercase ascii character");

        match s.chars().count() {
            2 => std::array::try_from_fn(next_char).map(Self::Alpha2),
            3 => std::array::try_from_fn(next_char).map(Self::Alpha3),
            n => Err(Error::InvalidLength(n)),
        }
    }
}

impl Display for Territory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alpha2([a, b]) => write!(f, "{a}{b}"),
            Self::Alpha3([a, b, c]) => write!(f, "{a}{b}{c}"),
            Self::Numeric(n) => n.fmt(f),
        }
    }
}

/// Parses a locale from a command-line argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocaleValueParser;

impl TypedValueParser for LocaleValueParser {
    type Value = Locale;

    fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, clap::Error> {
        let inner = clap::value_parser!(Box<str>);
        let value = inner.parse_ref(cmd, arg, value)?;

        value.parse().map_err(|_| clap::Error::new(clap::error::ErrorKind::ValueValidation))
    }
}

/// Returns the next character from the given iterator, ensuring that it matches the given predicate.
///
/// # Errors
///
/// This function will return an error if the character is missing or does not match the predicate.
fn match_next_char<I, P>(iterator: &mut I, predicate: P, expecting: &'static str) -> Result<char, Error>
where
    I: Iterator<Item = char>,
    P: FnOnce(&char) -> bool,
{
    iterator.next().ok_or(Error::MissingCharacter(expecting)).and_then(|character| {
        predicate(&character).then_some(character).ok_or(Error::InvalidCharacter(expecting, character))
    })
}
