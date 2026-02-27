// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024-2026 Jaxydog
// Copyright © 2025 RemasteredArch
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

use std::num::NonZero;
use std::path::PathBuf;

use clap::{Args, ValueEnum};
use ina_macro::optional;
use serde::{Deserialize, Serialize};
use supports_color::Stream;

/// The bot's settings.
#[non_exhaustive]
#[optional(
    keep_annotations = [non_exhaustive],
    apply_derives = [Clone, Debug, Hash, PartialEq, Eq],
    apply_annotations = {
        #[expect(clippy::struct_excessive_bools, reason = "not relevant to CLI arguments")]
    },
)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Args, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[group(id = "BotSettings")]
pub struct Settings {
    /// The location of the file that determines the bot's status.
    ///
    /// Default: `./res/status.toml`
    #[arg(long = "status-file")]
    #[option(default = self::default_status_file())]
    pub status_file: PathBuf,
    /// The interval at which to refresh the bot's status in minutes.
    ///
    /// Default: `30`
    #[arg(short = 'S', long = "status-interval")]
    #[option(default = self::default_status_interval())]
    pub status_interval: NonZero<u64>,

    /// The location of the directory holding attachment overrides for the `/help` command.
    ///
    /// Some of the buttons on the `/help` response trigger messages with attachments. These attachments are embedded
    /// into the bot, but it will look for files of the same name in this directory before defaulting to the embedded
    /// copy.
    ///
    /// Default: `./res/attachments`
    #[arg(long = "help-attachments-directory")]
    #[option(default = self::default_help_attachments_directory())]
    pub help_attachments_directory: PathBuf,

    /// The number of shards to spawn.
    ///
    /// Default: the Discord API's recommendation (see
    /// <https://discord.com/developers/docs/events/gateway#get-gateway-bot>)
    #[arg(short = 's', long = "shards")]
    #[option(default)]
    pub shards: Option<NonZero<u32>>,
    /// The interval at which to reshard in hours.
    ///
    /// Default: `8`
    #[arg(short = 'r', long = "reshard-interval")]
    #[option(default = self::default_reshard_interval())]
    pub reshard_interval: NonZero<u64>,

    /// Whether to skip command patching on bot startup.
    ///
    /// Default: `false`
    #[arg(long = "skip-command-patching")]
    #[option(default)]
    pub skip_command_patch: bool,

    /// Whether to output color during logging.
    ///
    /// Default: `true` if color is supported by standard output.
    #[arg(long = "color")]
    #[option(default)]
    pub color: ColorChoice,
    /// Disables all logger output.
    ///
    /// Equivalent to `--disable-file-logging` and `--disable-console-logging`.
    ///
    /// Default: `true` if `--disable-file-logging` and `--disable-console-logging` are `true`, `false` otherwise
    #[arg(short = 'q', long = "quiet")]
    #[option(default)]
    pub quiet: bool,
    /// Stops the logger from writing to files.
    ///
    /// Default: the value of `--quiet` (default `false`)
    #[arg(long = "disable-file-logging")]
    #[option(default)]
    pub disable_file_logging: bool,
    /// Stops the logger from writing to `STDOUT` and `STDERR`.
    ///
    /// Default: the value of `--quiet` (default `false`)
    #[arg(long = "disable-console-logging")]
    #[option(default)]
    pub disable_console_logging: bool,
    /// The logger's file output directory.
    ///
    /// Default: `./log`
    #[arg(id = "LOG_DIR", long = "log-directory")]
    #[option(default = self::default_log_directory())]
    pub log_directory: PathBuf,
}

/// Determines whether color should be output during logging.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum ColorChoice {
    /// Automatically output color if it is supported by the terminal's standard output stream.
    #[default]
    Auto,
    /// Always output color.
    Always,
    /// Never output color.
    Never,
}

impl ColorChoice {
    /// Returns `true` if color should be supported on the given stream.
    #[must_use]
    pub fn is_supported_on(self, stream: Stream) -> bool {
        use supports_color::on_cached;

        (matches!(self, Self::Auto) && on_cached(stream).is_some_and(|c| c.has_basic)) || matches!(self, Self::Always)
    }
}

/// Returns the default status file location.
fn default_status_file() -> PathBuf {
    std::env::current_dir().map_or_else(|_| PathBuf::from("./res/status.toml"), |v| v.join("res/status.toml"))
}

/// Returns the default re-sharding interval.
fn default_reshard_interval() -> NonZero<u64> {
    let Some(interval) = NonZero::new(8) else { unreachable!("the default interval must be non-zero") };

    interval
}

/// Returns the default help attachments directory location.
fn default_help_attachments_directory() -> PathBuf {
    std::env::current_dir().map_or_else(|_| PathBuf::from("./res/attachments"), |v| v.join("res/attachments"))
}

/// Returns the default status change interval.
fn default_status_interval() -> NonZero<u64> {
    let Some(interval) = NonZero::new(30) else { unreachable!("the default interval must be non-zero") };

    interval
}

/// Returns the default log directory.
fn default_log_directory() -> PathBuf {
    std::env::current_dir().map_or_else(|_| PathBuf::from("./log/"), |v| v.join("log"))
}
