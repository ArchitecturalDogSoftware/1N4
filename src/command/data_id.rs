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

use anyhow::{bail, ensure, Result};

/// The inner type of the [`DataId`] identifier strings.
pub type Inner = Arc<str>;

/// A custom identifier that supports arbitrary data storage.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DataId<I = Inner>
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

impl<I> DataId<I>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    /// The maximum length of a finished identifier in bytes.
    pub const MAX_LENGTH: usize = 100;
    /// The byte that separates individual sections of the identifier.
    pub const SECTION_SEPARATOR: char = '$';
    /// The byte that separates individual data sections.
    pub const VALUE_SEPARATOR: char = ';';

    /// Creates a new [`DataId<I>`].
    #[inline]
    pub fn new(name: impl AsRef<str>, kind: impl AsRef<str>) -> Self {
        Self { name: name.as_ref().into(), kind: kind.as_ref().into(), data: vec![] }
    }

    /// Returns a reference to the command name of this [`DataId<I>`].
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the component name of this [`DataId<I>`].
    #[inline]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns a reference to the data of this [`DataId<I>`].
    #[inline]
    pub fn data(&self) -> &[Box<str>] {
        &self.data
    }

    /// Adds the given string into the data section of this identifier.
    #[inline]
    pub fn push(&mut self, data: impl AsRef<str>) {
        self.data.push(data.as_ref().into());
    }

    /// Adds the given string into the data section of this identifier.
    ///
    /// This method is an alias for [`Self::push`] that allows chaining.
    #[inline]
    #[must_use]
    pub fn with(mut self, data: impl AsRef<str>) -> Self {
        self.push(data);
        self
    }

    /// Ensures that the constructed string will be valid.
    ///
    /// # Errors
    ///
    /// This function will return an error if the identifier is not valid.
    pub fn validate(self) -> Result<Self> {
        let data_sep_len = Self::VALUE_SEPARATOR.len_utf8() * self.data.len().saturating_sub(1);
        let data_len = self.data.iter().map(|s| s.len()).sum::<usize>() + data_sep_len;
        let full_sep_len = Self::SECTION_SEPARATOR.len_utf8() * 2;
        let full_len = self.name.len() + self.kind.len() + data_len + full_sep_len;

        ensure!(full_len < Self::MAX_LENGTH, "maximum length exceeded ({}/{} bytes)", full_len, Self::MAX_LENGTH);

        Ok(self)
    }
}

impl<I> From<DataId<I>> for String
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    #[inline]
    fn from(value: DataId<I>) -> Self {
        value.to_string()
    }
}

impl<I> Display for DataId<I>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = &(*self.name);
        let kind = &(*self.kind);
        let data = self.data.join(&Self::VALUE_SEPARATOR.to_string());

        write!(f, "{name}{s}{kind}{s}{data}", s = Self::SECTION_SEPARATOR)
    }
}

impl<I> TryFrom<String> for DataId<I>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        <Self as FromStr>::from_str(&value)
    }
}

impl<I> TryFrom<&str> for DataId<I>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        <Self as FromStr>::from_str(value)
    }
}

impl<I> FromStr for DataId<I>
where
    I: Deref<Target = str> + for<'s> From<&'s str>,
{
    type Err = anyhow::Error;

    fn from_str(string: &str) -> std::result::Result<Self, Self::Err> {
        let mut parts = string.split(Self::SECTION_SEPARATOR).take(3);

        let Some(name) = parts.next() else { bail!("missing command name") };
        let Some(kind) = parts.next() else { bail!("missing component name") };
        let Some(data) = parts.next() else { bail!("missing identifier data") };

        let mut identifier = Self::new(name, kind);

        for string in data.split(Self::VALUE_SEPARATOR) {
            identifier.push(string);
        }

        identifier.validate()
    }
}
