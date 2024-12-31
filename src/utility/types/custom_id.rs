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

use std::fmt::{Debug, Display};
use std::str::FromStr;
use std::sync::Arc;

use twilight_validate::component::COMPONENT_CUSTOM_ID_LENGTH;

/// An error that may be returned when interacting with custom identifiers.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Returned if a part of the identifier is missing during parsing.
    #[error("missing identifier {0}")]
    MissingPart(&'static str),
    /// Returned if the identifier's command name is considered malformed.
    #[error("invalid identifier command '{0}'")]
    InvalidCommand(Arc<str>),
    /// Returned if the identifier's variant name is considered malformed.
    #[error("invalid identifier variant '{0}'")]
    InvalidVariant(Arc<str>),
    /// Returned if the identifier's stored data contains an invalid character.
    #[error("invalid identifier data '{0}' contains unexpected character {1:?}")]
    InvalidData(Arc<str>, char),
    /// Returned if the identifier's maximum allowed length is exceeded.
    #[error("maximum length exceeded ({0}/{COMPONENT_CUSTOM_ID_LENGTH} bytes)")]
    ExceededMaxLength(usize),
}

/// A custom identifier that supports arbitrary data storage.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CustomId {
    /// The source command name.
    command: Arc<str>,
    /// The source command variant name.
    variant: Arc<str>,
    /// The identifier's data storage.
    storage: Vec<Arc<str>>,
}

impl CustomId {
    /// The character that separates individual parts of the identifier's data.
    pub const DATA_SEPARATOR: char = '\u{C}';
    /// The character that separates individual parts of the identifier.
    pub const PART_SEPARATOR: char = '\0';

    /// Creates a new [`CustomId`] with the given command and variant names.
    ///
    /// # Errors
    ///
    /// This function will return an error if the created identifier is considered invalid.
    pub fn new(command: impl AsRef<str>, variant: impl AsRef<str>) -> Result<Self, Error> {
        let this = Self { command: command.as_ref().into(), variant: variant.as_ref().into(), storage: Vec::new() };

        this.ensure_valid().map(move |()| this)
    }

    /// Returns a reference to the command name of this [`CustomId`].
    #[must_use]
    pub const fn command(&self) -> &Arc<str> {
        &self.command
    }

    /// Returns a reference to the variant name of this [`CustomId`].
    #[must_use]
    pub const fn variant(&self) -> &Arc<str> {
        &self.variant
    }

    /// Returns a reference to the data storage of this [`CustomId`].
    #[must_use]
    pub const fn storage(&self) -> &Vec<Arc<str>> {
        &self.storage
    }

    /// Returns a reference to the string stored at the given index within this [`CustomId`].
    #[must_use]
    pub fn get_str(&self, index: usize) -> Option<&Arc<str>> {
        self.storage.get(index)
    }

    /// Returns the value stored at the given index within this [`CustomId`], attempting to parse it if it exists.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be parsed.
    #[must_use]
    pub fn get<T: FromStr>(&self, index: usize) -> Option<Result<T, T::Err>> {
        self.get_str(index).map(|v| v.parse())
    }

    /// Adds the given data string into this [`CustomId`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the new identifier would be considered invalid.
    pub fn push_str(&mut self, data: impl AsRef<str>) -> Result<(), Error> {
        self.storage.push(data.as_ref().into());

        self.ensure_valid().inspect_err(|_| drop(self.storage.pop()))
    }

    /// Adds the given value into this [`CustomId`] as a parseable string.
    ///
    /// # Errors
    ///
    /// This function will return an error if the new identifier would be considered invalid.
    pub fn push<T>(&mut self, data: T) -> Result<(), Error>
    where
        T: Into<String> + FromStr,
    {
        self.push_str(data.into())
    }

    /// Adds the given data string into this [`CustomId`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the new identifier would be considered invalid.
    pub fn with_str(mut self, data: impl AsRef<str>) -> Result<Self, Error> {
        self.push_str(data).map(move |()| self)
    }

    /// Adds the given value into this [`CustomId`] as a parseable string.
    ///
    /// # Errors
    ///
    /// This function will return an error if the new identifier would be considered invalid.
    pub fn with<T>(self, data: T) -> Result<Self, Error>
    where
        T: Into<String> + FromStr,
    {
        self.with_str(data.into())
    }

    /// Returns a result indicating whether this [`CustomId`] is 'valid'.
    ///
    /// The identifier is considered valid if its total length is less than or equal to Discord's identifier character
    /// limit.
    /// There are also some additional restrictions placed upon the `command` and `variant` fields that ensure that they
    /// could be considered to be valid 1N4 command identifiers.
    /// This function *also* ensures that none of the strings contained within the `storage` list contain a separator
    /// character.
    ///
    /// # Errors
    ///
    /// This function will return an error if the identifier is not valid.
    fn ensure_valid(&self) -> Result<(), Error> {
        let data_sep_len = Self::DATA_SEPARATOR.len_utf8() * self.storage.len().saturating_sub(1);
        let data_len = self.storage.iter().map(|v| v.len()).sum::<usize>() + data_sep_len;
        let total_len = self.command.len() + self.variant.len() + data_len + (Self::PART_SEPARATOR.len_utf8() * 2);

        if total_len > COMPONENT_CUSTOM_ID_LENGTH {
            return Err(Error::ExceededMaxLength(total_len));
        }

        if !self.command.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_')) {
            return Err(Error::InvalidCommand(Arc::clone(&self.command)));
        }
        if !self.variant.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_')) {
            return Err(Error::InvalidVariant(Arc::clone(&self.command)));
        }

        if let Some((s, c)) = self.storage.iter().find_map(|s| {
            [Self::DATA_SEPARATOR, Self::PART_SEPARATOR].into_iter().find_map(|c| s.contains(c).then_some((s, c)))
        }) {
            return Err(Error::InvalidData(Arc::clone(s), c));
        }

        Ok(())
    }
}

impl From<&CustomId> for String {
    fn from(value: &CustomId) -> Self {
        value.to_string()
    }
}

impl From<&mut CustomId> for String {
    fn from(value: &mut CustomId) -> Self {
        value.to_string()
    }
}

impl From<CustomId> for String {
    fn from(value: CustomId) -> Self {
        value.to_string()
    }
}

impl Display for CustomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let storage = self.storage.join(&Self::DATA_SEPARATOR.to_string());

        write!(f, "{}{PS}{}{PS}{storage}", self.command, self.variant, PS = Self::PART_SEPARATOR)
    }
}

impl TryFrom<&str> for CustomId {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        <Self as FromStr>::from_str(value)
    }
}

impl TryFrom<&String> for CustomId {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        <Self as FromStr>::from_str(value)
    }
}

impl TryFrom<&mut String> for CustomId {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &mut String) -> Result<Self, Self::Error> {
        <Self as FromStr>::from_str(value)
    }
}

impl TryFrom<String> for CustomId {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        <Self as FromStr>::from_str(&value)
    }
}

impl FromStr for CustomId {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut parts = string.splitn(3, Self::PART_SEPARATOR);

        let Some(command) = parts.next() else { return Err(Error::MissingPart("command")) };
        let Some(variant) = parts.next() else { return Err(Error::MissingPart("variant")) };
        let Some(storage) = parts.next() else { return Err(Error::MissingPart("storage")) };

        let mut identifier = Self::new(command, variant)?;

        for data_str in storage.split(Self::DATA_SEPARATOR) {
            identifier.push_str(data_str)?;
        }

        Ok(identifier)
    }
}
