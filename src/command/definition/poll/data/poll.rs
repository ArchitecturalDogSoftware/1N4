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

use std::fmt::Write;
use std::num::NonZeroU16;

use anyhow::Result;
use ina_localizing::locale::Locale;
use ina_localizing::localize;
use ina_macro::{AsTranslation, Stored};
use ina_storage::format::{Compress, Messagepack};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use twilight_model::channel::message::Embed;
use twilight_model::id::Id;
use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

use super::input::PollInput;
use super::response::PollResponse;
use crate::command::definition::poll::data::input::{
    HybridInputData, MultipleChoiceInputData, OpenResponseInputData, RaffleInputData,
};
use crate::utility::category;
use crate::utility::traits::convert::AsTranslation;

/// A poll's type.
#[non_exhaustive]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, AsTranslation)]
#[serde(rename_all = "kebab-case")]
#[localizer_category(category::UI)]
pub enum PollType {
    /// A multiple-choice poll.
    #[localizer_key("poll-multiple-choice")]
    MultipleChoice,
    /// An open-response poll.
    #[localizer_key("poll-open-response")]
    OpenResponse,
    /// A multiple-choice poll with an open-ended option.
    #[localizer_key("poll-hybrid")]
    Hybrid,
    /// A raffle poll.
    #[localizer_key("poll-raffle")]
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
    /// The poll's image URL.
    pub image_url: Option<Box<str>>,

    /// The poll's submission period duration.
    pub duration: NonZeroU16,

    /// The poll's inputs.
    pub inputs: Vec<PollInput>,
}

impl PollBuilder {
    pub async fn build_preview(&self, locale: Option<Locale>) -> Result<Embed> {
        macro_rules! field {
            ($content:expr, $locale:expr, $key:literal, $value:expr) => {{
                let key = localize!(async(try in $locale) category::UI, $key).await?;

                writeln!(&mut $content, "**{key}:** {}", $value)?;
            }};
        }

        let embed_title = localize!(async(try in locale) category::UI, "poll-builder-title").await?;
        let mut builder = EmbedBuilder::new().title(embed_title);
        let mut content = String::new();

        field!(&mut content, locale, "poll-builder-title-field", self.title);
        field!(
            &mut content,
            locale,
            "poll-builder-type-field",
            format_args!("{} {}", self.kind.emoji(), self.kind.as_translation(locale).await?)
        );
        field!(&mut content, locale, "poll-builder-duration-field", {
            let minutes = f64::from(self.duration.get());

            let (time, unit) = if minutes < 60.0 {
                (minutes, localize!(async(try in locale) category::UI, "unit-minutes").await?)
            } else if minutes < 60.0 * 60.0 {
                (minutes / 60.0, localize!(async(try in locale) category::UI, "unit-hours").await?)
            } else {
                (minutes / (60.0 * 60.0), localize!(async(try in locale) category::UI, "unit-days").await?)
            };

            format!("{time} {unit}")
        });

        if let Some(description) = self.description.as_deref() {
            field!(
                &mut content,
                locale,
                "poll-builder-description-field",
                format_args!("\n>>> {}", description.replace('\n', "\n>>> "))
            );
        }

        if !self.inputs.is_empty() {
            field!(&mut content, locale, "poll-builder-inputs-field", "\n");

            for input in &self.inputs {
                write!(&mut content, "- ")?;

                match input {
                    PollInput::MultipleChoice(MultipleChoiceInputData { name, .. })
                    | PollInput::OpenResponse(OpenResponseInputData { name, .. })
                    | PollInput::Hybrid(
                        HybridInputData::MultipleChoice(MultipleChoiceInputData { name, .. })
                        | HybridInputData::OpenResponse(OpenResponseInputData { name, .. }),
                    ) => {
                        writeln!(&mut content, "{name}")?;
                    }
                    PollInput::Raffle(RaffleInputData { winners }) => {
                        let text = localize!(async(try in locale) category::UI, "poll-builder-winners-field").await?;

                        writeln!(&mut content, "{winners} {text}")?;
                    }
                }
            }
        }

        if let Some(image_url) = self.image_url.as_deref() {
            builder = builder.image(ImageSource::url(image_url)?);
        }

        builder.description(content).validate().map(EmbedBuilder::build).map_err(Into::into)
    }
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
    pub duration: NonZeroU16,
    /// The poll's starting time.
    pub start_time: OffsetDateTime,
    /// The poll's expected ending time.
    pub ending_time: OffsetDateTime,

    /// The poll's inputs.
    pub inputs: Box<[PollInput]>,
    /// The poll's responses.
    pub responses: Vec<PollResponse>,
}
