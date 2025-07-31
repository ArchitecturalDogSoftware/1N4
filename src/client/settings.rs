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
use std::path::PathBuf;

use clap::Args;
use serde::{Deserialize, Serialize};

/// The bot's settings.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Args, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[group(id = "BotSettings")]
pub struct Settings {
    /// The location of the file that determines the bot's status.
    #[arg(long = "status-file", default_value_os_t = self::default_status_file())]
    #[serde(default = "default_status_file")]
    pub status_file: PathBuf,
    /// The interval at which to refresh the bot's status in minutes.
    #[arg(short = 'S', long = "status-interval", default_value_t = self::default_status_interval())]
    #[serde(default = "default_status_interval")]
    pub status_interval: NonZeroU64,

    /// The location of the directory holding attachment overrides for the `/help` command.
    ///
    /// Some of the buttons on the `/help` response trigger messages with attachments. These
    /// attachments are embedded into the bot, but it will look for files of the same name in this
    /// directory before defaulting to the embedded copy.
    #[arg(long = "help-attachments-directory", default_value_os_t = self::default_help_attachments_directory())]
    #[serde(default = "default_help_attachments_directory")]
    pub help_attachments_directory: PathBuf,

    /// The number of shards to spawn.
    #[arg(short = 's', long = "shards")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shards: Option<NonZeroU32>,
    /// The interval at which to reshard in hours.
    #[arg(short = 'r', long = "reshard-interval", default_value_t = self::default_reshard_interval())]
    #[serde(default = "default_reshard_interval")]
    pub reshard_interval: NonZeroU64,

    /// Whether to skip command patching on bot startup.
    #[arg(long = "skip-command-patching")]
    #[serde(default)]
    pub skip_command_patch: bool,
}

/// Returns the default status file location.
fn default_status_file() -> PathBuf {
    std::env::current_dir().map_or_else(|_| PathBuf::from("./res/status.toml"), |v| v.join("res/status.toml"))
}

/// Returns the default re-sharding interval.
fn default_reshard_interval() -> NonZeroU64 {
    let Some(interval) = NonZeroU64::new(8) else { unreachable!("the default interval must be non-zero") };

    interval
}

/// Returns the default help attachments directory location.
fn default_help_attachments_directory() -> PathBuf {
    std::env::current_dir().map_or_else(|_| PathBuf::from("./res/attachments"), |v| v.join("res/attachments"))
}

/// Returns the default status change interval.
fn default_status_interval() -> NonZeroU64 {
    let Some(interval) = NonZeroU64::new(30) else { unreachable!("the default interval must be non-zero") };

    interval
}
