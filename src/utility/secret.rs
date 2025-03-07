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

use std::num::NonZeroU64;
use std::sync::Arc;

use anyhow::Result;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker};

/// Returns the environment variable associated with the given key, if present.
///
/// # Errors
///
/// This function will return an error if the environment variable is not defined.
fn get(key: &str) -> Result<Arc<str>> {
    Ok(std::env::var(key)?.into())
}

/// Returns the identifier associated with the given key, if present.
///
/// # Errors
///
/// This function will return an error if the environment variable is not defined, or if it is an invalid identifier.
fn get_id<T>(key: &str) -> Result<Id<T>> {
    Ok(std::env::var(key)?.parse::<NonZeroU64>()?.into())
}

/// Returns the Discord token environment variable, if present.
///
/// This can be configured using `DISCORD_TOKEN`.
///
/// # Errors
///
/// This function will return an error if the environment variable is not defined.
pub fn discord_token() -> Result<Arc<str>> {
    self::get("DISCORD_TOKEN")
}

/// Returns the development guild identifier environment variable, if present.
///
/// This can be configured using `DEVELOPMENT_GUILD_ID`.
///
/// # Errors
///
/// This function will return an error if the environment variable is not defined.
pub fn development_guild_id() -> Result<Id<GuildMarker>> {
    self::get_id("DEVELOPMENT_GUILD_ID")
}

/// Returns the development channel identifier environment variable, if present.
///
/// This can be configured using `DEVELOPMENT_CHANNEL_ID`.
///
/// # Errors
///
/// This function will return an error if the environment variable is not defined.
pub fn development_channel_id() -> Result<Id<ChannelMarker>> {
    self::get_id("DEVELOPMENT_CHANNEL_ID")
}

/// Returns the Discord token environment variable, if present.
///
/// This can be configured using `ENCRYPTION_KEY`.
///
/// # Errors
///
/// This function will return an error if the environment variable is not defined.
pub fn encryption_key() -> Result<Arc<str>> {
    self::get("ENCRYPTION_KEY")
}
