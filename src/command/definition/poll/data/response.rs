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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use twilight_model::id::Id;
use twilight_model::id::marker::UserMarker;

/// A poll's response.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollResponse {
    /// The user's identifier.
    pub user_id: Id<UserMarker>,
    /// The response's creation date.
    pub created_at: OffsetDateTime,
    /// The response data.
    pub data: PollResponseData,
}

/// A poll's response data.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PollResponseData {
    /// A multiple-choice poll response.
    MultipleChoice(MultipleChoiceResponseData),
    /// An open-response poll response.
    OpenResponse(OpenResponseResponseData),
    /// An response for a multiple-choice poll with an open-ended option.
    Hybrid(HybridResponseData),
    /// A raffle poll response.
    Raffle(RaffleResponseData),
}

/// Defines multiple choice response data.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipleChoiceResponseData {
    /// The input index.
    pub index: u8,
}

/// Defines open response response data.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenResponseResponseData {
    /// A map of input indexes to their responses.
    pub responses: BTreeMap<u8, Option<Box<str>>>,
}

/// Defines hybrid response data.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum HybridResponseData {
    /// A multiple choice response.
    MultipleChoice(MultipleChoiceResponseData),
    /// An open response response.
    OpenResponse(OpenResponseResponseData),
}

/// Defines raffle response data.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaffleResponseData {
    /// The input index.
    pub index: u8,
}
