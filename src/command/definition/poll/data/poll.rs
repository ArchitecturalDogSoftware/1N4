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

use ina_macro::{AsTranslation, Stored};
use ina_storage::format::{Compress, Messagepack};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_model::id::Id;

use super::input::PollInput;
use super::response::PollResponse;
use crate::utility::category;

/// A poll's type.
#[non_exhaustive]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, AsTranslation)]
#[serde(rename_all = "kebab-case")]
#[localizer_category(category::UI)]
pub enum PollType {
    /// A multiple-choice poll.
    #[localizer_key("multiple-choice")]
    MultipleChoice,
    /// An open-response poll.
    #[localizer_key("open-response")]
    OpenResponse,
    /// A multiple-choice poll with an open-ended option.
    #[localizer_key("hybrid")]
    Hybrid,
    /// A raffle poll.
    #[localizer_key("raffle")]
    Raffle,
}

impl PollType {
    /// Returns the emoji that represents this poll type.
    pub const fn emoji(self) -> char {
        match self {
            Self::MultipleChoice => '🔢',
            Self::OpenResponse => '📝',
            Self::Hybrid => '🔠',
            Self::Raffle => '🎲',
        }
    }
}

/// Builds a poll.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Stored)]
#[data_format(kind = Compress<Messagepack>, from = Compress::new_fast(Messagepack))]
#[data_path(fmt = "poll/builder/{}/{}", args = [Id<GuildMarker>, Id<UserMarker>], from = [guild_id, user_id])]
pub struct PollBuilder {
    /// The identifier of the poll author.
    pub user_id: Id<UserMarker>,
    /// The identifier of the poll guild.
    pub guild_id: Id<GuildMarker>,

    /// The poll's type.
    pub kind: PollType,
    /// The poll's title.
    pub title: Box<str>,
    /// The poll's description.
    pub description: Option<Box<str>>,

    /// The poll's submission period duration.
    pub duration: NonZeroU64,

    /// The poll's inputs.
    pub inputs: Vec<PollInput>,
}

/// Tracks an active poll's state.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Stored)]
#[data_format(kind = Compress<Messagepack>, from = Compress::new_default(Messagepack))]
#[data_path(fmt = "poll/state/{}/{}", args = [Id<GuildMarker>, Id<UserMarker>], from = [guild_id, user_id])]
pub struct PollState {
    /// The identifier of the poll author.
    pub user_id: Id<UserMarker>,
    /// The identifier of the poll guild.
    pub guild_id: Id<GuildMarker>,

    /// The poll's type.
    pub kind: PollType,
    /// The poll's title.
    pub title: Box<str>,
    /// The poll's description.
    pub description: Option<Box<str>>,

    /// The poll's submission period duration.
    pub duration: NonZeroU64,
    /// The poll's starting time.
    pub start_time: OffsetDateTime,
    /// The poll's expected ending time.
    pub ending_time: OffsetDateTime,

    /// The poll's inputs.
    pub inputs: Box<[PollInput]>,
    /// The poll's responses.
    pub responses: Vec<PollResponse>,
}
