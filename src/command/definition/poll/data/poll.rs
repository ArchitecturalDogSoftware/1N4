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

use serde::{Deserialize, Serialize};
use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_model::id::Id;

use super::input::PollInput;

/// A poll's type.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PollType {
    /// A multiple-choice poll.
    MultipleChoice,
    /// An open-response poll.
    OpenResponse,
    /// A multiple-choice poll with an open-ended option.
    Hybrid,
    /// A raffle poll.
    Raffle,
}

impl PollType {
    /// Returns the stringified name of the poll type.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MultipleChoice => "multiple-choice",
            Self::OpenResponse => "open-response",
            Self::Hybrid => "hybrid",
            Self::Raffle => "raffle",
        }
    }
}

/// Builds a poll.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollState {
    /// The identifier of the poll author.
    pub user_id: Id<UserMarker>,
    /// The identifier of the poll guild.
    pub guild_id: Id<GuildMarker>,
}
