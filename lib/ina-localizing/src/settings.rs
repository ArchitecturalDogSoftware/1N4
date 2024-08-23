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

use std::num::NonZeroUsize;
use std::path::Path;

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::locale::Locale;
use crate::text::TextRef;
use crate::{Error, Result};

/// The localizer's settings.
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
    #[arg(long = "lang-miss-behavior", default_value = "return")]
    #[serde(rename = "miss-behavior")]
    pub miss_behavior: MissingBehavior,

    /// The localizing thread's output queue capacity. If set to '1', no buffering will be done.
    #[arg(id = "LANG_QUEUE_CAPACITY", long = "lang-queue-capacity", default_value = "8")]
    #[serde(rename = "queue-capacity")]
    pub queue_capacity: NonZeroUsize,
}

/// The behavior to follow when the localizer is unable to translate a key.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MissingBehavior {
    /// Returns the missing text.
    Return,
    /// Returns an error.
    Error,
}

impl MissingBehavior {
    /// Calls the missing behavior.
    ///
    /// # Errors
    ///
    /// This function will return an error if the miss behavior specifies that outcome.
    pub fn call<'tx: 'fc, 'fc>(&self, category: &'fc str, key: &'fc str) -> Result<TextRef<'tx, 'fc>> {
        match self {
            Self::Return => Ok(TextRef::Missing(category, key)),
            Self::Error => Err(Error::MissingText(category.into(), key.into())),
        }
    }
}
