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

use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;

use clap::Args;
use ina_macro::optional;
use serde::{Deserialize, Serialize};

/// The logger's settings.
#[non_exhaustive]
#[optional(
    keep_annotations = [non_exhaustive, expect],
    apply_derives = [Clone, Debug, Hash, PartialEq, Eq, Serialize],
)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Args, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[group(id = "LogSettings")]
pub struct Settings {
    /// The logger's file output directory.
    ///
    /// Default: `./log`
    #[cfg(feature = "file")]
    #[arg(id = "LOG_DIR", long = "log-directory")]
    #[option(default = self::default_directory())]
    pub directory: PathBuf,

    /// The capacity of the logger's queue. If set to `1`, no buffering will occur.
    ///
    /// Default: `8`
    #[arg(id = "LOG_QUEUE_LEN", long = "log-queue-capacity")]
    #[option(default = self::default_queue_capacity())]
    pub queue_capacity: NonZeroUsize,
    /// The duration in milliseconds that the logger's queue should retain entries for before flushing.
    ///
    /// Default: `10`
    #[arg(id = "LOG_QUEUE_MS", long = "log-queue-duration")]
    #[option(default = self::default_queue_duration())]
    pub queue_duration: NonZeroU64,
}

/// Returns the default queue capacity.
fn default_queue_capacity() -> NonZeroUsize {
    let Some(capacity) = NonZeroUsize::new(8) else { unreachable!("the default capacity must be non-zero") };

    capacity
}

/// Returns the default queue duration.
fn default_queue_duration() -> NonZeroU64 {
    let Some(duration) = NonZeroU64::new(10) else { unreachable!("the default duration must be non-zero") };

    duration
}

/// Returns the default log directory.
#[cfg(feature = "file")]
fn default_directory() -> PathBuf {
    std::env::current_dir().map_or_else(|_| PathBuf::from("./log/"), |v| v.join("log"))
}
