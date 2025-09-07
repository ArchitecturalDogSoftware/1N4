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
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::path::PathBuf;

use clap::{Args, ValueEnum};
use ina_macro::optional;
use serde::{Deserialize, Serialize};

use crate::locale::Locale;
use crate::text::Text;
use crate::{Error, Result};

/// The localizer's settings.
#[non_exhaustive]
#[optional(
    keep_derives = [Clone, Debug, PartialEq, Eq, Serialize],
    keep_annotations = [non_exhaustive, expect],
)]
#[derive(Clone, Debug, PartialEq, Eq, Args, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[group(id = "LangSettings")]
pub struct Settings {
    /// The localizer's default locale.
    #[arg(short = 'l', long = "default-locale")]
    #[option(default)]
    pub default_locale: Locale,

    /// The directory within which to read language files.
    #[arg(id = "LANG_DIRECTORY", long = "lang-directory")]
    #[option(default = self::default_directory())]
    pub directory: PathBuf,

    /// The behavior that the localizer will exhibit when it fails to translate a key.
    #[arg(long = "lang-miss-behavior")]
    #[option(default)]
    pub miss_behavior: MissingBehavior,

    /// The localizing thread's output queue capacity. If set to '1', no buffering will be done.
    #[arg(id = "LANG_QUEUE_CAPACITY", long = "lang-queue-capacity")]
    #[option(default = self::default_queue_capacity())]
    pub queue_capacity: NonZeroUsize,

    /// The amount of depth at which to search for a translation key in language files with inherited translations.
    #[arg(id = "LANG_SEARCH_DEPTH", long = "lang-search-depth")]
    #[option(default = self::default_search_depth())]
    pub search_depth: usize,
}

/// The behavior to follow when the localizer is unable to translate a key.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MissingBehavior {
    /// Returns a string that is representative of the missing key.
    #[cfg_attr(not(debug_assertions), default)]
    Return,
    /// Returns an error.
    #[cfg_attr(debug_assertions, default)]
    Error,
}

impl MissingBehavior {
    /// Calls the missing behavior.
    ///
    /// # Errors
    ///
    /// This function will return an error if the miss behavior specifies that outcome.
    pub fn call<'tx: 'fc, 'fc, I>(&self, category: &'fc str, key: &'fc str) -> Result<Text<I>>
    where
        I: Deref<Target = str> + for<'a> From<&'a str>,
    {
        match self {
            Self::Return => Ok(Text::Missing(category.into(), key.into())),
            Self::Error => Err(Error::MissingText(category.into(), key.into())),
        }
    }
}

impl Display for MissingBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some(value) = self.to_possible_value() else { unreachable!("no variants are marked as skipped") };

        f.write_str(value.get_name())
    }
}

/// Returns the default queue capacity.
fn default_queue_capacity() -> NonZeroUsize {
    let Some(capacity) = NonZeroUsize::new(8) else { unreachable!("the default capacity must be non-zero") };

    capacity
}

/// Returns the default language file directory.
fn default_directory() -> PathBuf {
    std::env::current_dir().map_or_else(|_| PathBuf::from("./res/lang/"), |v| v.join("res/lang"))
}

/// Returns the default recursive search depth.
const fn default_search_depth() -> usize {
    2
}
