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

use serde::Serialize;

use crate::locale::Locale;

/// An owned translation key.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize)]
pub enum OwnedTranslation<T>
where
    T: Deref<Target = str>,
{
    /// The key is present in the initial map.
    Present(T),
    /// The key is present in an inherited map.
    Inherit(Locale, T),
    /// The key is missing and was returned.
    Missing(T, T),
}

impl<T> OwnedTranslation<T>
where
    T: Deref<Target = str>,
{
    /// Returns whether this [`OwnedTranslation<T>`] is [`OwnedTranslation::Present`].
    #[must_use]
    pub const fn is_present(&self) -> bool {
        matches!(self, Self::Present(..))
    }

    /// Returns whether this [`OwnedTranslation<T>`] is [`OwnedTranslation::Inherit`].
    #[must_use]
    pub const fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit(..))
    }

    /// Returns whether this [`OwnedTranslation<T>`] is [`OwnedTranslation::Missing`].
    #[must_use]
    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing(..))
    }

    /// Converts from an [`OwnedTranslation<T>`] into an [`OwnedTranslation<U>`].
    pub fn cast<U>(self) -> OwnedTranslation<U>
    where
        U: Deref<Target = str> + for<'s> From<&'s str>,
    {
        match self {
            Self::Present(v) => OwnedTranslation::Present((&(*v)).into()),
            Self::Inherit(l, v) => OwnedTranslation::Inherit(l, (&(*v)).into()),
            Self::Missing(c, k) => OwnedTranslation::Missing((&(*c)).into(), (&(*k)).into()),
        }
    }

    /// Returns the translation with the most highly-defined 'presence'.
    ///
    /// If this is [`OwnedTranslation::Present`], this is always returned.
    /// If this is [`OwnedTranslation::Missing`], `other` is always returned.
    /// If this is [`OwnedTranslation::Inherit`], `other` will be returned if it is [`OwnedTranslation::Present`].
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        match self {
            Self::Missing(..) => other,
            Self::Inherit(..) if other.is_present() => other,
            value => value,
        }
    }

    /// Returns the translation with the most highly-defined 'presence'.
    ///
    /// The given closure will only be evaluated if it's required to determine what to return.
    ///
    /// If this is [`OwnedTranslation::Present`], this is always returned.
    /// If this is [`OwnedTranslation::Missing`], `f` is always returned.
    /// If this is [`OwnedTranslation::Inherit`], `f` will be returned if it is [`OwnedTranslation::Present`].
    #[must_use]
    pub fn or_else(self, f: impl FnOnce() -> Self) -> Self {
        match self {
            Self::Present(_) => self,
            Self::Missing(..) => f(),
            Self::Inherit(..) => {
                let other = f();

                if other.is_present() { other } else { self }
            }
        }
    }

    /// Returns a borrow of this [`OwnedTranslation`].
    #[must_use]
    pub fn as_borrowed(&self) -> Translation {
        match self {
            Self::Present(v) => Translation::Present(v),
            Self::Inherit(l, v) => Translation::Inherit(*l, v),
            Self::Missing(c, k) => Translation::Missing(c, k),
        }
    }
}

impl<T> From<OwnedTranslation<T>> for String
where
    T: Deref<Target = str>,
{
    fn from(value: OwnedTranslation<T>) -> Self {
        value.to_string()
    }
}

impl<T> Display for OwnedTranslation<T>
where
    T: Deref<Target = str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Present(v) | Self::Inherit(_, v) => write!(f, "{}", &Deref::deref(v)),
            Self::Missing(c, k) => write!(f, "{}::{}", &Deref::deref(c), &Deref::deref(k)),
        }
    }
}

/// A borrowed translation key.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize)]
pub enum Translation<'lc: 'ag, 'ag> {
    /// The key is present in the initial map.
    Present(&'lc str),
    /// The key is present in an inherited map.
    Inherit(Locale, &'lc str),
    /// The key is missing and was returned.
    Missing(&'ag str, &'ag str),
}

impl<'lc: 'ag, 'ag> Translation<'lc, 'ag> {
    /// Returns whether this [`Translation`] is [`Translation::Present`].
    #[must_use]
    pub const fn is_present(&self) -> bool {
        matches!(self, Self::Present(..))
    }

    /// Returns whether this [`Translation`] is [`Translation::Inherit`].
    #[must_use]
    pub const fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit(..))
    }

    /// Returns whether this [`Translation`] is [`Translation::Missing`].
    #[must_use]
    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing(..))
    }

    /// Returns the translation with the most highly-defined 'presence'.
    ///
    /// If this is [`Translation::Present`], this is always returned.
    /// If this is [`Translation::Missing`], `other` is always returned.
    /// If this is [`Translation::Inherit`], `other` will be returned if it is [`Translation::Present`].
    #[must_use]
    pub const fn or(self, other: Self) -> Self {
        match self {
            Self::Missing(..) => other,
            Self::Inherit(..) if other.is_present() => other,
            value => value,
        }
    }

    /// Returns the translation with the most highly-defined 'presence'.
    ///
    /// The given closure will only be evaluated if it's required to determine what to return.
    ///
    /// If this is [`Translation::Present`], this is always returned.
    /// If this is [`Translation::Missing`], `f` is always returned.
    /// If this is [`Translation::Inherit`], `f` will be returned if it is [`Translation::Present`].
    #[must_use]
    pub fn or_else(self, f: impl FnOnce() -> Self) -> Self {
        match self {
            Self::Present(_) => self,
            Self::Missing(..) => f(),
            Self::Inherit(..) => {
                let other = f();

                if other.is_present() { other } else { self }
            }
        }
    }

    /// Returns an owned version of this [`Translation`].
    #[must_use]
    pub fn as_owned<'tr, T>(&'tr self) -> OwnedTranslation<T>
    where
        T: Deref<Target = str> + From<&'tr str>,
    {
        match self {
            Self::Present(v) => OwnedTranslation::Present((*v).into()),
            Self::Inherit(l, v) => OwnedTranslation::Inherit(*l, (*v).into()),
            Self::Missing(c, k) => OwnedTranslation::Missing((*c).into(), (*k).into()),
        }
    }
}

impl Display for Translation<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Present(v) | Self::Inherit(_, v) => write!(f, "{v}"),
            Self::Missing(c, k) => write!(f, "{c}::{k}"),
        }
    }
}
