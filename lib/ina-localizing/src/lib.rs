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

#![feature(array_try_from_fn)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thread::Request;
use tokio::sync::RwLock;

use self::locale::Locale;
use self::settings::{MissingBehavior, Settings};
use self::text::TextRef;

/// Defines the format for locales.
pub mod locale;
/// Defines the localizer's settings.
pub mod settings;
/// Defines translated text.
pub mod text;
/// Defines the library's thread implementation.
pub mod thread;

/// A result alias with a defaulted error type.
pub type Result<T> = std::result::Result<T, Error>;

/// An error that may be returned when using this library.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A locale-specific error.
    #[error(transparent)]
    Locale(#[from] self::locale::Error),
    /// The configured directory is missing.
    #[error("missing configured directory: '{0}'")]
    MissingDir(Box<Path>),
    /// A file is missing for the given locale.
    #[error("missing language file for locale: '{0}'")]
    MissingFile(Locale),
    /// A missing or invalid text was requested.
    #[error("missing text for key: '{0}::{1}'")]
    MissingText(Box<str>, Box<str>),
    /// A locale was missing.
    #[error("an expected locale was missing")]
    MissingLocale,
    /// A [`get_recursive`](<Language::get_recursive>) call exceeded its specified limit.
    #[error("recursion limit exceeded")]
    RecursionLimit,
    /// A TOML deserialization error.
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    /// An error from communicating with a thread.
    #[allow(clippy::type_complexity)]
    #[error(transparent)]
    Thread(#[from] ina_threading::Error<(Option<usize>, (Arc<RwLock<Localizer>>, Request))>),
}

/// A value that stores and retrieves translated text.
#[derive(Clone, Debug)]
pub struct Localizer {
    /// The localizer's settings.
    settings: Settings,
    /// The localizer's stored locales and their assigned language data.
    languages: HashMap<Locale, Language>,
}

impl Localizer {
    /// Creates a new [`Localizer`].
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        Self { settings, languages: HashMap::new() }
    }

    /// Returns the loaded locales of this [`Localizer`].
    pub fn locales(&self) -> impl Iterator<Item = Locale> + '_ {
        self.languages.keys().copied()
    }

    /// Returns whether this [`Localizer`] has loaded the given locale.
    #[must_use]
    pub fn has_locale(&self, locale: &Locale) -> bool {
        self.languages.contains_key(locale)
    }

    /// Clears the specified locales if they have been loaded, clearing all locales if given [`None`].
    pub fn clear_locales(&mut self, locales: Option<impl IntoIterator<Item = Locale>>) {
        if let Some(locales) = locales.map(|l| l.into_iter().collect::<Box<[_]>>()) {
            self.languages.retain(|l, _| !locales.contains(l));
        } else {
            self.languages.clear();
        }
    }

    /// Attempts to load the language file for the given locale.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file does not exist or the operation fails.
    pub async fn load_locale(&mut self, locale: Locale) -> Result<()> {
        let path = self.settings.directory.join(locale.to_string()).with_extension("toml");

        if !tokio::fs::try_exists(&path).await? {
            return Err(Error::MissingFile(locale));
        }

        let text = tokio::fs::read_to_string(path).await?;
        let language = toml::from_str(&text)?;

        self.languages.insert(locale, language);

        Ok(())
    }

    /// Attempts to load the language files for the given locales.
    ///
    /// # Errors
    ///
    /// This function will return an error if a file does not exist or any of the operations fail.
    pub async fn load_locales<I>(&mut self, locales: I) -> Result<usize>
    where
        I: IntoIterator<Item = Locale> + Send,
        I::IntoIter: Send,
    {
        let mut count = 0;

        for locale in locales {
            self.load_locale(locale).await?;

            count += 1;
        }

        Ok(count)
    }

    /// Attempts to load the configured directory of this [`Localizer`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the directory is missing or any of the operations fail.
    pub async fn load_directory(&mut self) -> Result<usize> {
        let path = &(*self.settings.directory);

        if !tokio::fs::try_exists(path).await? {
            return Err(Error::MissingDir(path.into()));
        }

        let mut iterator = tokio::fs::read_dir(path).await?;
        let mut locales = Vec::new();

        while let Some(entry) = iterator.next_entry().await? {
            let metadata = entry.metadata().await?;

            if !metadata.is_file() {
                continue;
            }

            let path = entry.path();

            let Some(name) = path.file_stem() else {
                continue;
            };

            if let Ok(locale) = name.to_string_lossy().parse() {
                locales.push(locale);
            }
        }

        self.load_locales(locales).await
    }

    /// Returns the translated text for the given key.
    ///
    /// # Errors
    ///
    /// This function will return an error if the text is not found and the configured behavior specifies to return an
    /// error.
    pub fn get<'tx: 'fc, 'fc>(
        &'tx self,
        locale: Locale,
        category: &'fc str,
        key: &'fc str,
    ) -> Result<TextRef<'tx, 'fc>> {
        let Some(language) = self.languages.get(&locale) else {
            return if self.settings.default_locale == locale {
                self.settings.miss_behavior.call(category, key)
            } else {
                self.get(self.settings.default_locale, category, key)
            };
        };

        language.get_recursive(category, key, self.settings.miss_behavior, &self.languages, Language::DEFAULT_MAX_DEPTH)
    }
}

/// Defines and stores the contents of a language file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Language {
    /// The locale that this language inherits from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherit: Option<Locale>,
    /// The language's defined text categories and their defined keys.
    #[serde(default, flatten, skip_serializing_if = "HashMap::is_empty")]
    pub categories: HashMap<Box<str>, HashMap<Box<str>, Box<str>>>,
}

impl Language {
    /// The default amount of depth at which to search for a key before giving up.
    const DEFAULT_MAX_DEPTH: usize = 4;

    /// Returns the text for a key within the given category as written within this language file.
    ///
    /// # Errors
    ///
    /// This function will return an error if the text is not present and the behavior specifies to return an error.
    pub fn get<'tx: 'fc, 'fc>(
        &'tx self,
        category: &'fc str,
        key: &'fc str,
        behavior: MissingBehavior,
    ) -> Result<TextRef<'tx, 'fc>> {
        self.categories
            .get(category)
            .and_then(|k| k.get(key))
            .map_or_else(|| behavior.call(category, key), |s| Ok(TextRef::Present(s)))
    }

    /// Returns the text for a key within the given category as written within this or a parent language file.
    ///
    /// # Errors
    ///
    /// This function will return an error if the text is not present and the behavior specifies to return an error.
    pub fn get_recursive<'tx: 'fc, 'fc>(
        &'tx self,
        category: &'fc str,
        key: &'fc str,
        behavior: MissingBehavior,
        languages: &'tx HashMap<Locale, Self>,
        max_depth: usize,
    ) -> Result<TextRef<'tx, 'fc>> {
        if max_depth == 0 {
            return behavior.call(category, key).map_err(|_| Error::RecursionLimit);
        }

        let text = self.get(category, key, behavior);

        // Only continue if the resolved text is missing.
        let (Ok(TextRef::Missing(..)) | Err(Error::MissingText(..))) = text else {
            return text;
        };

        // Resolve the parent language.
        let Some(ref locale) = self.inherit else {
            return text;
        };
        let Some(parent) = languages.get(locale) else {
            return text;
        };

        // Convert `Present` variants to `Inherit` variants.
        match parent.get_recursive(category, key, behavior, languages, max_depth - 1) {
            Ok(TextRef::Present(value)) => Ok(TextRef::Inherit(*locale, value)),
            other => other,
        }
    }
}
