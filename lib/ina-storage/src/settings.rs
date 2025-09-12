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
use std::path::PathBuf;

use clap::Args;
use ina_macro::optional;
use serde::{Deserialize, Serialize};

use crate::System;

/// The storage instance's settings.
#[non_exhaustive]
#[optional(
    keep_annotations = [non_exhaustive, expect],
    apply_derives = [Clone, Debug, Hash, PartialEq, Eq, Serialize],
)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Args, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[group(id = "DataSettings")]
pub struct Settings {
    /// The storage system to use to read and write data.
    #[arg(long = "data-system")]
    #[option(default)]
    pub system: System,
    /// The directory within which to manage data files.
    #[arg(id = "DATA_DIRECTORY", long = "data-directory")]
    #[option(default = self::default_directory())]
    pub directory: PathBuf,

    /// The storage thread's output queue capacity. If set to '1', no buffering will be done.
    #[arg(id = "DATA_QUEUE_CAPACITY", long = "data-queue-capacity")]
    #[option(default = self::default_queue_capacity())]
    pub queue_capacity: NonZeroUsize,
}

/// Returns the default queue capacity.
fn default_queue_capacity() -> NonZeroUsize {
    let Some(capacity) = NonZeroUsize::new(8) else { unreachable!("the default capacity must be non-zero") };

    capacity
}

/// Returns the default data directory.
fn default_directory() -> PathBuf {
    std::env::current_dir().map_or_else(|_| PathBuf::from("./res/data/"), |v| v.join("res/data"))
}
