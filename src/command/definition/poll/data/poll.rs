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

use anyhow::{Result, bail};
use ina_localizing::locale::Locale;
use ina_localizing::localize;
use ina_macro::{AsTranslation, Stored};
use ina_storage::format::{Compress, Messagepack};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use tokio_stream::{Stream, StreamExt};
use twilight_model::channel::message::component::ButtonStyle;
use twilight_model::channel::message::{Component, Embed, EmojiReactionType};
use twilight_model::id::Id;
use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_model::user::User;
use twilight_util::builder::embed::{EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, ImageSource};
use twilight_validate::embed::FIELD_VALUE_LENGTH;

use super::input::PollInput;
use super::response::PollResponse;
use crate::command::definition::poll::data::input::{
    HybridInputData, MultipleChoiceInputData, OpenResponseInputData, RaffleInputData,
};
use crate::command::registry::CommandEntry;
use crate::utility::category;
use crate::utility::traits::convert::{AsEmoji, AsTranslation};
use crate::utility::types::builder::ButtonBuilder;
use crate::utility::types::id::CustomId;

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

/// A poll.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize, Stored)]
#[data_format(kind = Compress<Messagepack>, from = Compress::new_fast(Messagepack))]
#[data_path(fmt = "poll/{}/{}", args = [Id<GuildMarker>, Id<UserMarker>], from = [guild_id, user_id])]
pub struct Poll {
    /// The identifier of the poll author.
    pub user_id: Id<UserMarker>,
    /// The identifier of the poll guild.
    pub guild_id: Id<GuildMarker>,

    /// The poll's title.
    pub title: Box<str>,
    /// The poll's optional description.
    pub about: Option<Box<str>>,
    /// The poll's optional image URL.
    pub image: Option<Box<str>>,

    /// The poll's type.
    pub kind: PollType,
    /// The poll's duration in minutes.
    pub minutes: NonZeroU16,
    /// The poll's state.
    pub state: PollState,
}

impl Poll {
    /// Builds the poll, creating an embed and message components that represent its current state.
    ///
    /// # Errors
    ///
    /// This function will return an error if the poll could not be built.
    pub async fn build(
        &self,
        entry: &CommandEntry,
        locale: Option<Locale>,
        user: &User,
        page: Option<usize>,
    ) -> Result<(Embed, Box<[Component]>)> {
        Ok((self.build_embed(entry, locale, user, page).await?, self.build_components(entry, locale, page).await?))
    }

    /// Builds the poll's embed, which represents its current state.
    ///
    /// # Errors
    ///
    /// This function will return an error if the poll's embed could not be built.
    async fn build_embed(
        &self,
        entry: &CommandEntry,
        locale: Option<Locale>,
        user: &User,
        page: Option<usize>,
    ) -> Result<Embed> {
        match &self.state {
            PollState::Builder { .. } => self.build_embed_for_builder(locale, user).await,
            PollState::Running { .. } => todo!(),
            PollState::Archive { .. } => todo!(),
        }
    }

    /// Builds the poll's builder embed.
    ///
    /// # Errors
    ///
    /// This function will return an error if the poll's embed could not be built.
    async fn build_embed_for_builder(&self, locale: Option<Locale>, user: &User) -> Result<Embed> {
        let PollState::Builder { inputs } = &self.state else {
            bail!("expected poll state to be variant `PollState::Builder`");
        };

        let header = localize!(async(try in locale) category::UI, "poll-builder-header").await?;
        let mut embed = EmbedBuilder::new().author(EmbedAuthorBuilder::new(header)).title(&(*self.title));

        if let Some(about) = self.about.as_deref() {
            embed = embed.description(about);
        }

        if let Some(image) = self.image.as_deref() {
            embed = embed.image(ImageSource::url(image)?);
        }

        if let Some(color) = user.accent_color {
            embed = embed.color(color);
        } else {
            embed = embed.color(crate::utility::color::BRANDING_B);
        }

        let type_field = EmbedFieldBuilder::new(
            localize!(async(try in locale) category::UI, "poll-builder-type").await?,
            format!("{} {}", self.kind.emoji(), self.kind.as_translation(locale).await?),
        )
        .inline();

        let duration_field = EmbedFieldBuilder::new(
            localize!(async(try in locale) category::UI, "poll-builder-duration").await?,
            (Duration::MINUTE * self.minutes.get()).to_string(),
        )
        .inline();

        let mut inputs_text = if self.kind == PollType::Raffle {
            format!("{}", inputs.len())
        } else {
            inputs.iter().filter_map(PollInput::label).collect::<Box<[_]>>().join(", ")
        };

        // The field value length assumes UTF-16, a two-byte-per-code-point system.
        // Since we're comparing directly against a byte count, this is fine.
        if inputs_text.len() > FIELD_VALUE_LENGTH * 2 {
            const ELLIPSIS: &str = "...";

            inputs_text.truncate((FIELD_VALUE_LENGTH * 2) - ELLIPSIS.len());
            inputs_text += ELLIPSIS;
        }

        let inputs_field = EmbedFieldBuilder::new(
            localize!(async(try in locale) category::UI, "poll-builder-inputs").await?,
            inputs_text,
        );

        embed = embed.field(type_field).field(duration_field).field(inputs_field);

        Ok(embed.validate()?.build())
    }

    /// Builds the poll's components, which represent its current state.
    ///
    /// # Errors
    ///
    /// This function will return an error if the poll's components could not be built.
    async fn build_components(
        &self,
        entry: &CommandEntry,
        locale: Option<Locale>,
        page: Option<usize>,
    ) -> Result<Box<[Component]>> {
        let mut components: Box<dyn Stream<Item = Result<Component>> + Unpin> = match &self.state {
            PollState::Builder { .. } => Box::from(self.build_components_for_builder(entry, locale)),
            PollState::Running { .. } => todo!(),
            PollState::Archive { .. } => todo!(),
        };

        while let Some(component) = components.try_next().await? {}

        todo!()
    }

    fn build_components_for_builder<'pl>(
        &'pl self,
        entry: &'pl CommandEntry,
        locale: Option<Locale>,
    ) -> impl Stream<Item = Result<Component>> + Unpin + 'pl {
        #[inline]
        async fn button(
            this: &Poll,
            name: &'static str,
            style: ButtonStyle,
            emoji: impl Into<EmojiReactionType> + Send,
            disabled: bool,
            entry: &CommandEntry,
            locale: Option<Locale>,
        ) -> Result<Component> {
            let key = format!("{}-builder-{name}", entry.name);
            let label = localize!(async(try in locale) category::UI_BUTTON, key).await?;
            let id = CustomId::<Box<str>>::new(entry.name, name)?
                .with(this.guild_id.to_string())?
                .with(this.user_id.to_string())?;

            Ok(ButtonBuilder::new(style).label(label)?.emoji(emoji)?.custom_id(id)?.disabled(disabled).build().into())
        }

        Box::pin(async_stream::try_stream! {
            yield button(self, "add-input", ButtonStyle::Primary, '➕'.as_emoji()?, false, entry, locale).await?;
            yield button(self, "remove-input", ButtonStyle::Primary, '➖'.as_emoji()?, false, entry, locale).await?;
        })
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PollState {
    /// The poll is currently being built.
    Builder {
        /// The poll's inputs.
        inputs: Vec<PollInput>,
    },
    /// The poll is actively running.
    Running {
        /// The poll's creation date.
        created: OffsetDateTime,
        /// The poll's inputs.
        inputs: Box<[PollInput]>,
        /// The poll's responses.
        responses: Vec<PollResponse>,
    },
    /// The poll has been archived.
    Archive {
        /// The poll's creation date.
        created: OffsetDateTime,
        /// The poll's archive date.
        archived: OffsetDateTime,
        /// The poll's inputs.
        inputs: Box<[PollInput]>,
        /// The poll's responses.
        responses: Box<[PollResponse]>,
    },
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

    /// The poll's submission period duration in minutes.
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
            } else if minutes < 60.0 * 24.0 {
                (minutes / 60.0, localize!(async(try in locale) category::UI, "unit-hours").await?)
            } else {
                (minutes / (60.0 * 24.0), localize!(async(try in locale) category::UI, "unit-days").await?)
            };

            format!("{time:.1} {unit}")
        });

        if let Some(description) = self.description.as_deref() {
            field!(&mut content, locale, "poll-builder-description-field", format_args!("\n>>> {}", description));
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
