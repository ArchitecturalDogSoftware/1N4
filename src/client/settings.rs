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

use std::num::{NonZeroU32, NonZeroU64};
use std::path::{Path, PathBuf};

use clap::Args;
use serde::{Deserialize, Serialize};

/// The bot's settings.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Args, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[group(id = "BotSettings")]
pub struct Settings {
    /// The location of the file that determines the bot's status.
    #[arg(long = "status-file", default_value = "./res/status.toml")]
    #[serde(default = "default_status_file")]
    pub status_file: Box<Path>,
    /// The interval at which to refresh the bot's status in minutes.
    #[arg(short = 'S', long = "status-interval", default_value = "30")]
    #[serde(default = "default_status_interval")]
    pub status_interval: NonZeroU64,

    /// The number of shards to spawn.
    #[arg(short = 's', long = "shards")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shards: Option<NonZeroU32>,
    /// The interval at which to reshard in hours.
    #[arg(short = 'r', long = "reshard-interval", default_value = "8")]
    #[serde(default = "default_reshard_interval")]
    pub reshard_interval: NonZeroU64,
}

/// Returns the default status file location.
fn default_status_file() -> Box<Path> {
    PathBuf::from("./res/status.toml").into_boxed_path()
}

/// Returns the default re-sharding interval.
const fn default_reshard_interval() -> NonZeroU64 {
    let Some(interval) = NonZeroU64::new(8) else { unreachable!() };

    interval
}

/// Returns the default status change interval.
const fn default_status_interval() -> NonZeroU64 {
    let Some(interval) = NonZeroU64::new(30) else { unreachable!() };

    interval
}
