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

use clap::Args;
use serde::{Deserialize, Serialize};

/// The logger's settings.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Args, Serialize, Deserialize)]
#[group(id = "LogSettings")]
pub struct Settings {
    /// Disables logger output.
    #[arg(short = 'q', long = "quiet")]
    #[serde(default)]
    pub quiet: bool,

    /// The capacity of the logger's queue. If set to '1', no buffering will occur.
    #[arg(id = "LOG_QUEUE_LEN", long = "log-queue-capacity", default_value = "8")]
    #[serde(default = "default_queue_capacity")]
    pub queue_capacity: NonZeroUsize,
    /// The duration in milliseconds that the logger's queue should retain entries for before flushing.
    #[arg(id = "LOG_QUEUE_MS", long = "log-queue-duration", default_value = "5")]
    #[serde(default = "default_queue_duration")]
    pub queue_duration: NonZeroU64,

    /// The logger's file output directory.
    #[cfg(feature = "file")]
    #[arg(id = "LOG_DIR", long = "log-directory", default_value = "./log/")]
    #[serde(default = "default_directory")]
    pub directory: Box<std::path::Path>,
}

/// Returns the default queue capacity.
const fn default_queue_capacity() -> NonZeroUsize {
    let Some(capacity) = NonZeroUsize::new(8) else { unreachable!() };

    capacity
}

/// Returns the default queue duration.
const fn default_queue_duration() -> NonZeroU64 {
    let Some(capacity) = NonZeroU64::new(10) else { unreachable!() };

    capacity
}

/// Returns the default log directory.
#[cfg(feature = "file")]
fn default_directory() -> Box<std::path::Path> {
    std::path::PathBuf::from("./log/").into_boxed_path()
}
