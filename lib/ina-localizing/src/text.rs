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
use std::sync::Arc;

use serde::Serialize;

use crate::locale::Locale;

/// The default type stored within an owned [`Text`] value.
pub type TextInner = Arc<str>;

/// A borrowed translation key.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize)]
pub enum TextRef<'tx: 'fc, 'fc, I = TextInner>
where
    I: Deref<Target = str> + for<'a> From<&'a str>,
{
    /// The text was present within the initial map.
    Present(&'tx I),
    /// The text was present within a parent map.
    Inherit(Locale, &'tx I),
    /// The text was not present.
    Missing(&'fc I, &'fc I),
}

impl<'tx: 'fc, 'fc, I> TextRef<'tx, 'fc, I>
where
    I: Deref<Target = str> + for<'a> From<&'a str>,
{
    /// Returns `true` if the text ref is [`Present`].
    ///
    /// [`Present`]: TextRef::Present
    #[must_use]
    pub const fn is_present(&self) -> bool {
        matches!(self, Self::Present(..))
    }

    /// Returns `true` if the text ref is [`Inherit`].
    ///
    /// [`Inherit`]: TextRef::Inherit
    #[must_use]
    pub const fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit(..))
    }

    /// Returns `true` if the text ref is [`Missing`].
    ///
    /// [`Missing`]: TextRef::Missing
    #[must_use]
    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing(..))
    }

    /// Returns the translation with the most 'highly-defined' value.
    ///
    /// If this is [`Inherit`], then this function returns `other` if `other` is [`Present`].
    /// If this is [`Missing`], then this function returns `other` if `other` is not [`Missing`].
    /// In all other cases, this value is returned.
    ///
    /// [`Present`]: TextRef::Present
    /// [`Inherit`]: TextRef::Inherit
    /// [`Missing`]: TextRef::Missing
    #[must_use]
    pub const fn or(self, other: Self) -> Self {
        match self {
            Self::Inherit(..) if other.is_present() => other,
            Self::Missing(..) if !other.is_missing() => other,
            this => this,
        }
    }

    /// Returns the translation with the most 'highly-defined' value, only calling the given function if necessary.
    ///
    /// If this is [`Inherit`], then this function returns `other` if `other` is [`Present`].
    /// If this is [`Missing`], then this function returns `other` if `other` is not [`Missing`].
    /// In all other cases, this value is returned.
    ///
    /// [`Present`]: TextRef::Present
    /// [`Inherit`]: TextRef::Inherit
    /// [`Missing`]: TextRef::Missing
    #[must_use]
    pub fn or_else(self, f: impl FnOnce() -> Self) -> Self {
        match self {
            Self::Inherit(..) | Self::Missing(..) => self.or(f()),
            this @ Self::Present(_) => this,
        }
    }

    /// Returns an owned version of this [`TextRef`].
    ///
    /// This may be a cheap or expensive conversion depending on the typing of the `I` generic.
    #[must_use]
    pub fn into_owned(self) -> Text<I>
    where
        I: Clone,
    {
        match self {
            Self::Present(t) => Text::Present(t.clone()),
            Self::Inherit(l, t) => Text::Inherit(l, t.clone()),
            Self::Missing(c, k) => Text::Missing(c.clone(), k.to_owned()),
        }
    }
}

impl<'tx: 'fc, 'fc, I> From<TextRef<'tx, 'fc, I>> for String
where
    I: Deref<Target = str> + for<'a> From<&'a str>,
{
    fn from(value: TextRef<'tx, 'fc, I>) -> Self {
        value.to_string()
    }
}

impl<'tx: 'fc, 'fc, I> Display for TextRef<'tx, 'fc, I>
where
    I: Deref<Target = str> + for<'a> From<&'a str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Present(t) | Self::Inherit(_, t) => write!(f, "{}", &(**t)),
            Self::Missing(c, k) => write!(f, "{}::{}", &(**c), &(**k)),
        }
    }
}

/// An owned translation key.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize)]
pub enum Text<I = TextInner>
where
    I: Deref<Target = str> + for<'a> From<&'a str>,
{
    /// The text was present within the initial map.
    Present(I),
    /// The text was present within a parent map.
    Inherit(Locale, I),
    /// The text was not present.
    Missing(I, I),
}

impl<I> Text<I>
where
    I: Deref<Target = str> + for<'a> From<&'a str>,
{
    /// Returns `true` if the text is [`Present`].
    ///
    /// [`Present`]: Text::Present
    #[must_use]
    pub const fn is_present(&self) -> bool {
        matches!(self, Self::Present(..))
    }

    /// Returns `true` if the text is [`Inherit`].
    ///
    /// [`Inherit`]: Text::Inherit
    #[must_use]
    pub const fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit(..))
    }

    /// Returns `true` if the text is [`Missing`].
    ///
    /// [`Missing`]: Text::Missing
    #[must_use]
    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing(..))
    }

    /// Converts the inner type of this [`Text`] from `I` to `U`.
    #[must_use]
    pub fn cast_inner<U>(self) -> Text<U>
    where
        U: Deref<Target = str> + for<'a> From<&'a str>,
    {
        match self {
            Self::Present(t) => Text::Present(U::from(&t)),
            Self::Inherit(l, t) => Text::Inherit(l, U::from(&t)),
            Self::Missing(c, k) => Text::Missing(U::from(&c), U::from(&k)),
        }
    }

    /// Returns the translation with the most 'highly-defined' value.
    ///
    /// If this is [`Inherit`], then this function returns `other` if `other` is [`Present`].
    /// If this is [`Missing`], then this function returns `other` if `other` is not [`Missing`].
    /// In all other cases, this value is returned.
    ///
    /// [`Present`]: Text::Present
    /// [`Inherit`]: Text::Inherit
    /// [`Missing`]: Text::Missing
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        match self {
            Self::Inherit(..) if other.is_present() => other,
            Self::Missing(..) if !other.is_missing() => other,
            this => this,
        }
    }

    /// Returns the translation with the most 'highly-defined' value, only calling the given function if necessary.
    ///
    /// If this is [`Inherit`], then this function returns `other` if `other` is [`Present`].
    /// If this is [`Missing`], then this function returns `other` if `other` is not [`Missing`].
    /// In all other cases, this value is returned.
    ///
    /// [`Present`]: Text::Present
    /// [`Inherit`]: Text::Inherit
    /// [`Missing`]: Text::Missing
    #[must_use]
    pub fn or_else(self, f: impl FnOnce() -> Self) -> Self {
        match self {
            Self::Inherit(..) | Self::Missing(..) => self.or(f()),
            this @ Self::Present(_) => this,
        }
    }

    /// Returns a borrowed version of this [`Text`].
    pub const fn as_borrowed(&self) -> TextRef<I> {
        match self {
            Self::Present(t) => TextRef::Present(t),
            Self::Inherit(l, t) => TextRef::Inherit(*l, t),
            Self::Missing(c, k) => TextRef::Missing(c, k),
        }
    }
}

impl<I> From<Text<I>> for String
where
    I: Clone + Deref<Target = str> + for<'a> From<&'a str>,
{
    fn from(value: Text<I>) -> Self {
        Self::from(value.as_borrowed())
    }
}

impl<I> Display for Text<I>
where
    I: Deref<Target = str> + for<'a> From<&'a str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_borrowed().fmt(f)
    }
}
