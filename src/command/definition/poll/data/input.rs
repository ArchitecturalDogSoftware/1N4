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

use std::num::NonZeroU8;

use serde::{Deserialize, Serialize};
use twilight_model::channel::message::EmojiReactionType;

/// Defines input count limits.
#[expect(dead_code, reason = "polls are currently a work-in-progress")]
pub mod limit {
    /// The maximum number of allowed multiple choice inputs.
    pub const MULTIPLE_CHOICE: usize = 20;
    /// The maximum number of allowed open response inputs.
    pub const OPEN_RESPONSE: usize = 20;
    /// The maximum number of allowed hybrid multiple choice inputs.
    pub const HYBRID_MULTIPLE_CHOICE: usize = MULTIPLE_CHOICE - 1;
    /// The maximum number of allowed hybrid open response inputs.
    pub const HYBRID_OPEN_RESPONSE: usize = OPEN_RESPONSE;
    /// The maximum number of allowed raffle inputs.
    pub const RAFFLE: usize = 5;
}

/// A poll's input.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PollInput {
    /// A multiple-choice poll input.
    MultipleChoice(MultipleChoiceInputData),
    /// An open-response poll input.
    OpenResponse(OpenResponseInputData),
    /// An input for a multiple-choice poll with an open-ended option.
    Hybrid(HybridInputData),
    /// A raffle poll input.
    Raffle(RaffleInputData),
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

/// Defines hybrid input data.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum HybridInputData {
    /// A multiple choice input.
    MultipleChoice(MultipleChoiceInputData),
    /// An open response input.
    OpenResponse(OpenResponseInputData),
}

/// Defines raffle input data.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaffleInputData {
    /// The number of members that can win this raffle.
    pub winners: NonZeroU8,
}
