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

use serde::{Deserialize, Serialize};
use twilight_model::channel::message::EmojiReactionType;

use super::poll::PollType;

/// A poll's input.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PollInput {
    /// A multiple-choice poll.
    MultipleChoice(MultipleChoiceInputData),
    /// An open-response poll.
    OpenResponse,
    /// A multiple-choice poll with an open-ended option.
    Hybrid,
    /// A raffle poll.
    Raffle,
}

impl PollInput {
    /// Returns the type of this [`PollInput`].
    #[must_use]
    pub const fn kind(&self) -> PollType {
        match self {
            Self::MultipleChoice(_) => PollType::MultipleChoice,
            Self::OpenResponse => PollType::OpenResponse,
            Self::Hybrid => PollType::Hybrid,
            Self::Raffle => PollType::Raffle,
        }
    }
}

/// Defines multiple choice input data.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipleChoiceInputData {
    /// The input's name.
    pub name: Box<str>,
    /// The input's icon.
    pub icon: Option<EmojiReactionType>,
}

/// Defines open response input data.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenResponseInputData {
    /// The input's name.
    pub name: Box<str>,
    /// The input's description.
    pub description: Option<Box<str>>,
}
