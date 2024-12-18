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
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

use twilight_validate::component::COMPONENT_CUSTOM_ID_LENGTH;

/// The byte that separates individual sections of the identifier.
pub const SECTION_SEPARATOR: char = '$';
/// The byte that separates individual data sections.
pub const VALUE_SEPARATOR: char = ';';

/// The inner type of the [`CustomId`] identifier strings.
pub type Inner = Arc<str>;

/// An error that may be returned when interacting with custom identifiers.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Returned when the identifier contains an invalid character.
    #[error("invalid custom identifier")]
    InvalidData,
    /// Returned when the command name is missing during parsing.
    #[error("missing command name")]
    MissingName,
    /// Returned when the component type is missing during parsing.
    #[error("missing component type")]
    MissingType,
    /// Returned when the data section is missing during parsing.
    #[error("missing data section")]
    MissingData,
    /// Returned when the identifier exceeds the maximum length.
    #[error("maximum length exceeded ({0}/{COMPONENT_CUSTOM_ID_LENGTH} bytes)")]
    MaximumLength(usize),
}

/// A custom identifier that supports arbitrary data storage.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct CustomId<I = Inner>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    /// The source command name.
    name: I,
    /// The component or modal name.
    kind: I,
    /// The stored data.
    data: Vec<Box<str>>,
}

impl<I> CustomId<I>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    /// Creates a new [`CustomId<I>`].
    ///
    /// # Errors
    ///
    /// The function will return an error if the identifier is invalid.
    pub fn new(name: impl AsRef<str>, kind: impl AsRef<str>) -> Result<Self, Error> {
        let output = Self { name: name.as_ref().into(), kind: kind.as_ref().into(), data: vec![] };

        output.try_validate()?;

        Ok(output)
    }

    /// Returns a reference to the command name of this [`CustomId<I>`].
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the component name of this [`CustomId<I>`].
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns a reference to the data of this [`CustomId<I>`].
    pub fn data(&self) -> &[Box<str>] {
        &self.data
    }

    /// Attempts to validate this [`CustomId<I>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the identifier is invalid.
    fn try_validate(&self) -> Result<(), Error> {
        // Currently, this calculates the length of the final string without allocating.
        // This is most likely more efficient than actually calling `.to_string()`, but this should be tested.
        let data_sep_len = VALUE_SEPARATOR.len_utf8() * self.data.len().saturating_sub(1);
        let data_len = self.data.iter().map(|s| s.len()).sum::<usize>() + data_sep_len;
        let len = self.name.len() + self.kind.len() + data_len + (SECTION_SEPARATOR.len_utf8() * 2);

        if len > COMPONENT_CUSTOM_ID_LENGTH {
            return Err(Error::MaximumLength(len));
        }
        if !self.name().chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_')) {
            return Err(Error::InvalidData);
        }
        if !self.kind().chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_')) {
            return Err(Error::InvalidData);
        }

        Ok(())
    }

    /// Adds the given string into the data section of this identifier.
    ///
    /// # Errors
    ///
    /// This function will return an error if the changed identifier is invalid.
    pub fn push(&mut self, data: impl AsRef<str>) -> Result<(), Error> {
        self.data.push(data.as_ref().into());

        self.try_validate()
    }

    /// Adds the given string into the data section of this identifier.
    ///
    /// This method is an alias for [`Self::push`] that allows chaining.
    ///
    /// # Errors
    ///
    /// This function will return an error if the changed identifier is invalid.
    pub fn with(mut self, data: impl AsRef<str>) -> Result<Self, Error> {
        self.push(data).map(move |()| self)
    }
}

impl<I> From<CustomId<I>> for String
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    fn from(value: CustomId<I>) -> Self {
        value.to_string()
    }
}

impl<I> Display for CustomId<I>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = &(*self.name);
        let kind = &(*self.kind);
        let data = self.data.join(&VALUE_SEPARATOR.to_string());

        write!(f, "{name}{SECTION_SEPARATOR}{kind}{SECTION_SEPARATOR}{data}")
    }
}

impl<I> TryFrom<String> for CustomId<I>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    type Error = <Self as FromStr>::Err;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        <Self as FromStr>::from_str(&value)
    }
}

impl<I> TryFrom<&str> for CustomId<I>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        <Self as FromStr>::from_str(value)
    }
}

impl<I> FromStr for CustomId<I>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut parts = string.split(SECTION_SEPARATOR).take(3);

        let Some(name) = parts.next() else { return Err(Error::MissingName) };
        let Some(kind) = parts.next() else { return Err(Error::MissingType) };
        let Some(data) = parts.next() else { return Err(Error::MissingData) };

        let mut identifier = Self::new(name, kind)?;

        for string in data.split(VALUE_SEPARATOR) {
            identifier.push(string)?;
        }

        Ok(identifier)
    }
}
