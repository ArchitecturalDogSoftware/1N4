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

/// Provides getters for client secrets.
pub mod secret;
/// Provides commonly-used trait definitions and blanket implementations.
pub mod traits;
/// Provides various useful types and builders.
pub mod types;

/// The base Discord CDN URL.
pub const DISCORD_CDN_URL: &str = "https://cdn.discordapp.com";
/// The base twemoji CDN URL.
pub const TWEMOJI_CDN_URL: &str = "https://raw.githubusercontent.com/discord/twemoji/main/assets/72x72";
