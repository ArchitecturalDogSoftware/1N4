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

//! Provides localization solutions for 1N4.

use std::collections::HashMap;
use std::convert::Infallible;
use std::num::NonZeroUsize;
use std::path::Path;

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::error::SendError;

pub use crate::locale::*;
pub use crate::translation::*;

/// Provides definitions for locales.
mod locale;
/// Contains the localizer's thread implementation.
pub mod thread;
/// Provides definitions for translations.
mod translation;

/// A result alias with a defaulted error type.
pub type Result<T, S = Infallible> = std::result::Result<T, Error<S>>;

/// An error that may occur when using this library.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error<S = Infallible> {
    /// A TOML deserializing error.
    #[error(transparent)]
    FromToml(#[from] toml::de::Error),
    /// An invalid locale was given.
    #[error("an invalid locale was provided")]
    InvalidLocale,
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A locale was missing.
    #[error("an requested locale was missing")]
    MissingLocale,
    /// A translation was missing.
    #[error("an requested translation was missing")]
    MissingTranslation,
    /// A sending error.
    #[error(transparent)]
    Send(#[from] SendError<S>),
    /// A threading error.
    #[error(transparent)]
    Threading(#[from] ina_threading::Error<S>),
}

/// The logger's settings.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Args, Serialize, Deserialize)]
#[group(id = "LangSettings")]
pub struct Settings {
    /// The localizer's default locale.
    #[arg(short = 'l', long = "default-locale", default_value = "en-US")]
    #[serde(rename = "default-locale")]
    pub default_locale: Locale,

    /// The directory within which to read language files.
    #[arg(id = "LANG_DIRECTORY", long = "lang-directory", default_value = "./res/lang/")]
    #[serde(rename = "directory")]
    pub file_directory: Box<Path>,

    /// The behavior that the localizer will exhibit when it fails to translate a key.
    #[arg(long = "lang-miss-behavior", default_value = "key")]
    #[serde(rename = "miss-behavior")]
    pub miss_behavior: MissBehavior,

    /// The localizing thread's output queue capacity. If set to '1', no buffering will be done.
    #[arg(id = "LANG_QUEUE_CAPACITY", long = "lang-queue-capacity", default_value = "8")]
    #[serde(rename = "queue-capacity")]
    pub queue_capacity: NonZeroUsize,
}

/// The behavior to follow when the localizer is unable to translate a key.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MissBehavior {
    /// Returns the raw translation key.
    Key,
    /// Returns an error.
    Error,
}

impl MissBehavior {
    /// Calls the miss behavior.
    ///
    /// # Errors
    ///
    /// This function will return an error if the miss behavior specifies that outcome.
    pub const fn call<'lc: 'ag, 'ag>(&self, category: &'ag str, key: &'ag str) -> Result<Translation<'lc, 'ag>> {
        match self {
            Self::Key => Ok(Translation::Missing(category, key)),
            Self::Error => Err(Error::MissingTranslation),
        }
    }
}

/// A localizer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Localizer {
    /// The localizer's settings.
    settings: Settings,
    /// The localizer's stored translations.
    locales: HashMap<Locale, Translations>,
}

impl Localizer {
    /// Creates a new [`Localizer`].
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        Self { settings, locales: HashMap::new() }
    }

    /// Returns the loaded locales of this [`Localizer`].
    pub fn locales(&self) -> impl Iterator<Item = Locale> + '_ {
        self.locales.keys().copied()
    }

    /// Returns whether this [`Localizer`] has loaded the given locale.
    #[must_use]
    pub fn has_locale(&self, locale: Locale) -> bool {
        self.locales.contains_key(&locale)
    }

    /// Clears all loaded locales.
    pub fn clear_locales(&mut self) {
        self.locales.clear();
    }

    /// Attempts to load the given locale.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to read the translation file.
    pub async fn load_locale(&mut self, locale: Locale) -> Result<()> {
        let path = self.settings.file_directory.join(locale.to_string()).with_extension("toml");

        if !tokio::fs::try_exists(&path).await? {
            return Err(Error::MissingLocale);
        }

        let data = tokio::fs::read_to_string(path).await?;
        let translations = toml::from_str(&data)?;

        self.locales.insert(locale, translations);

        Ok(())
    }

    /// Attempts to load the source directory of this [`Localizer`], returning the number of locales loaded.
    ///
    /// # Errors
    ///
    /// This function will return an error if the localizer fails to load a locale.
    pub async fn load_directory(&mut self) -> Result<usize> {
        let path = &(*self.settings.file_directory);

        if !tokio::fs::try_exists(path).await? {
            return Err(Error::MissingLocale);
        }

        let mut count: usize = 0;
        let mut iterator = tokio::fs::read_dir(path).await?;

        while let Some(entry) = iterator.next_entry().await? {
            let metadata = entry.metadata().await?;

            if metadata.is_file() {
                continue;
            }

            let path = entry.path();
            let Some(name) = path.file_stem() else { continue };

            if let Ok(locale) = name.to_string_lossy().parse() {
                self.load_locale(locale).await?;

                count += 1;
            }
        }

        Ok(count)
    }

    /// Returns a translation for the given key.
    ///
    /// # Errors
    ///
    /// This function will return an error if the key is not found and the provided mode specifies to return an error..
    pub fn get<'lc: 'ag, 'ag>(
        &'lc self,
        locale: Locale,
        category: &'ag str,
        key: &'ag str,
    ) -> Result<Translation<'lc, 'ag>> {
        let Some(translations) = self.locales.get(&locale) else {
            return self.settings.miss_behavior.call(category, key);
        };

        translations.get_inherited(self.settings.miss_behavior, &self.locales, category, key)
    }
}

/// The contents of a translation file.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Translations {
    /// The locale that this translation map inherits from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherit: Option<Locale>,
    /// Translations, sorted into categories.
    #[serde(default, flatten, skip_serializing_if = "HashMap::is_empty")]
    pub translations: HashMap<Box<str>, HashMap<Box<str>, Box<str>>>,
}

impl Translations {
    /// Returns the parent of this [`Translations`] map.
    #[must_use]
    pub const fn parent(&self) -> Option<Locale> {
        self.inherit
    }

    /// Returns a translation for a key as written within this specific map.
    ///
    /// This method does not check parent maps.
    ///
    /// # Errors
    ///
    /// This function will return an error if the key is not found and the provided mode specifies to return an error.
    pub fn get_defined<'lc: 'ag, 'ag>(
        &'lc self,
        mode: MissBehavior,
        category: &'ag str,
        key: &'ag str,
    ) -> Result<Translation<'lc, 'ag>> {
        let Some(map) = self.translations.get(category) else {
            return mode.call(category, key);
        };
        let Some(value) = map.get(key) else {
            return mode.call(category, key);
        };

        Ok(Translation::Present(value))
    }

    /// Returns a translation for a key as written within this or a parent map.
    ///
    /// # Errors
    ///
    /// This function will return an error if the key is not found and the provided mode specifies to return an error.
    pub fn get_inherited<'lc: 'ag, 'ag>(
        &'lc self,
        mode: MissBehavior,
        locales: &'lc HashMap<Locale, Self>,
        category: &'ag str,
        key: &'ag str,
    ) -> Result<Translation<'lc, 'ag>> {
        match self.get_defined(mode, category, key) {
            Ok(Translation::Inherit(..)) => unreachable!(),
            Ok(Translation::Missing(..)) | Err(Error::MissingTranslation) if self.parent().is_some() => {
                let Some(parent) = self.parent() else { unreachable!() };
                let Some(inherited) = locales.get(&parent) else { return mode.call(category, key) };

                match inherited.get_inherited(mode, locales, category, key) {
                    Ok(Translation::Present(value)) => Ok(Translation::Inherit(parent, value)),
                    value => value,
                }
            }
            value => value,
        }
    }
}
