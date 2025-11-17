// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2025 Jaxydog
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

//! Provides types that build off of the `ascii` crate.

use std::fmt::Display;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::str::FromStr;

use ascii::{AsciiChar, AsciiStr, ToAsciiChar, ToAsciiCharError};

/// An error returned when attempting to convert a value into an [`AsciiArray`].
#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct ToAsciiArrayError(ToAsciiArrayErrorKind);

impl ToAsciiArrayError {
    /// Return's this error's type.
    #[must_use]
    pub const fn kind(&self) -> ToAsciiArrayErrorKind {
        self.0
    }
}

impl std::error::Error for ToAsciiArrayError {}

impl Display for ToAsciiArrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A [`ToAsciiArrayError`]'s type.
#[derive(Clone, Copy, Debug)]
pub enum ToAsciiArrayErrorKind {
    /// A [`ToAsciiCharError`].
    ToAsciiChar(ToAsciiCharError),
    /// The converting type has an invalid length.
    InvalidLength(usize, usize),
}

impl Display for ToAsciiArrayErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToAsciiChar(error) => error.fmt(f),
            Self::InvalidLength(given, expected) => write!(f, "invalid length; given {given}, expected {expected}"),
        }
    }
}

/// An array of ASCII characters.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AsciiArray<const N: usize>([AsciiChar; N]);

impl<const N: usize> Deref for AsciiArray<N> {
    type Target = [AsciiChar; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for AsciiArray<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> Index<usize> for AsciiArray<N> {
    type Output = AsciiChar;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<const N: usize> IndexMut<usize> for AsciiArray<N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<const N: usize> IntoIterator for AsciiArray<N> {
    type IntoIter = std::array::IntoIter<Self::Item, N>;
    type Item = AsciiChar;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<const N: usize> From<[AsciiChar; N]> for AsciiArray<N> {
    fn from(value: [AsciiChar; N]) -> Self {
        Self(value)
    }
}

impl<const N: usize> TryFrom<[u8; N]> for AsciiArray<N> {
    type Error = ToAsciiCharError;

    fn try_from(value: [u8; N]) -> Result<Self, Self::Error> {
        value.try_map(ToAsciiChar::to_ascii_char).map(Self)
    }
}

impl<const N: usize> TryFrom<[char; N]> for AsciiArray<N> {
    type Error = ToAsciiCharError;

    fn try_from(value: [char; N]) -> Result<Self, Self::Error> {
        value.try_map(ToAsciiChar::to_ascii_char).map(Self)
    }
}

impl<const N: usize> TryFrom<&AsciiStr> for AsciiArray<N> {
    type Error = ToAsciiArrayError;

    fn try_from(value: &AsciiStr) -> Result<Self, Self::Error> {
        if value.len() != N {
            return Err(ToAsciiArrayError(ToAsciiArrayErrorKind::InvalidLength(value.len(), N)));
        }

        let mut array = [AsciiChar::Null; N];

        for (index, character) in value.chars().enumerate() {
            array[index] = character;
        }

        Ok(Self(array))
    }
}

impl<const N: usize> TryFrom<&str> for AsciiArray<N> {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl<const N: usize> FromStr for AsciiArray<N> {
    type Err = ToAsciiArrayError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if string.chars().count() != N {
            return Err(ToAsciiArrayError(ToAsciiArrayErrorKind::InvalidLength(string.chars().count(), N)));
        }

        let mut array = [AsciiChar::Null; N];

        for (index, character) in string.chars().enumerate() {
            array[index] = match character.to_ascii_char() {
                Ok(character) => character,
                Err(error) => return Err(ToAsciiArrayError(ToAsciiArrayErrorKind::ToAsciiChar(error))),
            };
        }

        Ok(Self(array))
    }
}
