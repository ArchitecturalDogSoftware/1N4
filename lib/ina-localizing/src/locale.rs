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

//! Defines regional linguistic locales.

use std::ffi::OsStr;
use std::fmt::Display;
use std::num::{NonZero, ParseIntError};
use std::str::FromStr;

use ascii::AsciiChar;
use clap::builder::{TypedValueParser, ValueParserFactory};
use clap::{Arg, Command};
use serde::de::{Unexpected, Visitor};
use serde::{Deserialize, Serialize};

use crate::ascii::{AsciiArray, ToAsciiArrayError};

/// An error returned when failing to create a [`Locale`].
#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct LocaleError(LocaleErrorKind);

impl LocaleError {
    /// Returns this error's type.
    #[must_use]
    pub const fn kind(&self) -> &LocaleErrorKind {
        &self.0
    }
}

impl std::error::Error for LocaleError {}

impl Display for LocaleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A [`LocaleError`]'s type.
#[derive(Clone, Debug)]
pub enum LocaleErrorKind {
    /// An invalid locale was provided.
    Locale(Box<str>),

    /// An invalid language code was provided.
    LanguageCode(AsciiArray<2>),

    /// An invalid alpha-2 territory code was provided.
    TerritoryAlpha2Code(AsciiArray<2>),
    /// An invalid alpha-3 territory code was provided.
    TerritoryAlpha3Code(AsciiArray<3>),
    /// An invalid numeric territory code was provided.
    TerritoryNumericCode(NonZero<u16>),

    /// A [`ParseIntError`].
    ParseInt(ParseIntError),
    /// A [`ToAsciiArrayError`].
    ToAsciiArray(ToAsciiArrayError),
}

impl Display for LocaleErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Locale(string) => write!(f, "an invalid locale string was provided: {string:?}"),
            Self::LanguageCode(code) => write!(f, "an invalid language code was provided: {code:?}"),
            Self::TerritoryAlpha2Code(code) => write!(f, "an invalid alpha-2 territory code was provided: {code:?}"),
            Self::TerritoryAlpha3Code(code) => write!(f, "an invalid alpha-3 territory code was provided: {code:?}"),
            Self::TerritoryNumericCode(code) => write!(f, "an invalid numeric territory code was provided: {code:?}"),
            Self::ParseInt(error) => write!(f, "failed to parse numeric territory code: {error}"),
            Self::ToAsciiArray(error) => write!(f, "failed to parse ascii array: {error}"),
        }
    }
}

/// The language code for a locale.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocaleLanguageCode(AsciiArray<2>);

impl LocaleLanguageCode {
    /// Creates a new [`LocaleLanguageCode`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the given code is not entirely ASCII lowercase.
    pub fn new(character_array: AsciiArray<2>) -> Result<Self, LocaleError> {
        if character_array.iter().all(AsciiChar::is_ascii_lowercase) {
            Ok(Self(character_array))
        } else {
            Err(LocaleError(LocaleErrorKind::LanguageCode(character_array)))
        }
    }
}

impl Display for LocaleLanguageCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.0[0], self.0[1])
    }
}

impl FromStr for LocaleLanguageCode {
    type Err = LocaleError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Self::new(string.parse().map_err(|error| LocaleError(LocaleErrorKind::ToAsciiArray(error)))?)
    }
}

/// The territory code for a locale.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocaleTerritoryCode(LocaleTerritoryCodeInner);

impl LocaleTerritoryCode {
    /// Creates a new alpha-2 [`LocaleTerritoryCode`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the given code is not entirely ASCII uppercase.
    pub fn alpha2(character_array: AsciiArray<2>) -> Result<Self, LocaleError> {
        if character_array.iter().all(AsciiChar::is_ascii_uppercase) {
            Ok(Self(LocaleTerritoryCodeInner::Alpha2(character_array)))
        } else {
            Err(LocaleError(LocaleErrorKind::TerritoryAlpha2Code(character_array)))
        }
    }

    /// Creates a new alpha-3 [`LocaleTerritoryCode`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the given code is not entirely ASCII uppercase.
    pub fn alpha3(character_array: AsciiArray<3>) -> Result<Self, LocaleError> {
        if character_array.iter().all(AsciiChar::is_ascii_uppercase) {
            Ok(Self(LocaleTerritoryCodeInner::Alpha3(character_array)))
        } else {
            Err(LocaleError(LocaleErrorKind::TerritoryAlpha3Code(character_array)))
        }
    }

    /// Creates a new numeric [`LocaleTerritoryCode`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the given code is not within `001..=999`.
    pub const fn numeric(number: NonZero<u16>) -> Result<Self, LocaleError> {
        if number.get() <= 999 {
            Ok(Self(LocaleTerritoryCodeInner::Numeric(number)))
        } else {
            Err(LocaleError(LocaleErrorKind::TerritoryNumericCode(number)))
        }
    }
}

impl Display for LocaleTerritoryCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for LocaleTerritoryCode {
    type Err = LocaleError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if string.chars().all(|character| character.is_ascii_digit()) {
            return Self::numeric(string.parse().map_err(|error| LocaleError(LocaleErrorKind::ParseInt(error)))?);
        }

        match string.chars().count() {
            2 => Self::alpha2(string.parse().map_err(|error| LocaleError(LocaleErrorKind::ToAsciiArray(error)))?),
            3 => Self::alpha3(string.parse().map_err(|error| LocaleError(LocaleErrorKind::ToAsciiArray(error)))?),
            _ => Err(LocaleError(LocaleErrorKind::Locale(string.into()))),
        }
    }
}

/// The inner representation of a [`LocaleTerritoryCode`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum LocaleTerritoryCodeInner {
    /// A two-letter regional identifier.
    Alpha2(AsciiArray<2>),
    /// A three-letter regional identifier.
    Alpha3(AsciiArray<3>),
    /// A numeric regional identifier, ranging from 001-999.
    Numeric(NonZero<u16>),
}

impl Display for LocaleTerritoryCodeInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alpha2(array) => array.iter().try_for_each(|character| character.fmt(f)),
            Self::Alpha3(array) => array.iter().try_for_each(|character| character.fmt(f)),
            Self::Numeric(number) => write!(f, "{number:03}"),
        }
    }
}

/// A regional linguistic locale.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Locale {
    /// The locale's language code.
    language: LocaleLanguageCode,
    /// The locale's territory identifier.
    territory: Option<LocaleTerritoryCode>,
}

impl Locale {
    /// Creates a new [`Locale`].
    #[must_use]
    pub const fn new(language: LocaleLanguageCode, territory: Option<LocaleTerritoryCode>) -> Self {
        Self { language, territory }
    }

    /// Returns the locale's language code.
    #[must_use]
    pub const fn language(&self) -> LocaleLanguageCode {
        self.language
    }

    /// Returns the locale's territory code.
    #[must_use]
    pub const fn territory(&self) -> Option<LocaleTerritoryCode> {
        self.territory
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::new(
            LocaleLanguageCode(AsciiArray::from([AsciiChar::e, AsciiChar::n])),
            Some(LocaleTerritoryCode(LocaleTerritoryCodeInner::Alpha2(AsciiArray::from([AsciiChar::U, AsciiChar::S])))),
        )
    }
}

impl Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(territory) = self.territory() {
            write!(f, "{}-{territory}", self.language())
        } else {
            self.language().fmt(f)
        }
    }
}

impl From<Locale> for String {
    fn from(value: Locale) -> Self {
        value.to_string()
    }
}

impl FromStr for Locale {
    type Err = LocaleError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if let Some((language, territory)) = string.split_once('-') {
            Ok(Self::new(language.parse()?, Some(territory.parse()?)))
        } else {
            Ok(Self::new(string.parse()?, None))
        }
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

/// Parses a locale from a command-line argument.
#[derive(Clone, Copy, Debug)]
pub struct LocaleValueParser;

impl TypedValueParser for LocaleValueParser {
    type Value = Locale;

    fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, clap::Error> {
        let inner = clap::value_parser!(Box<str>);
        let value = inner.parse_ref(cmd, arg, value)?;

        value.parse().map_err(|_| clap::Error::new(clap::error::ErrorKind::ValueValidation))
    }
}
