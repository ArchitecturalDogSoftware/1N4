// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024—2025 Jaxydog
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

use std::num::NonZeroU16;

use anyhow::bail;
use data::poll::{Poll, PollState, PollType};
use ina_localizing::localize;
use ina_storage::stored::Stored;
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::message::component::TextInputStyle;
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::InteractionResponseType;
use twilight_util::builder::embed::ImageSource;
use twilight_validate::embed::DESCRIPTION_LENGTH;

use crate::client::event::EventResult;
use crate::command::context::{Context, Visibility};
use crate::command::registry::CommandEntry;
use crate::command::resolver::{CommandOptionResolver, ModalFieldResolver};
use crate::utility::category;
use crate::utility::traits::convert::AsLocale;
use crate::utility::types::builder::TextInputBuilder;
use crate::utility::types::custom_id::CustomId;
use crate::utility::types::modal::ModalDataBuilder;

/// The command's data.
mod data {
    /// Defines input data.
    pub mod input;
    /// Defines poll data.
    pub mod poll;
    /// Defines response data.
    pub mod response;
}

crate::define_entry!("poll", CommandType::ChatInput, struct {
    // Until this command is finished, it will only be available in the linked development server.
    dev_only: true,
    contexts: [InteractionContextType::Guild],
    permissions: Permissions::SEND_POLLS,
}, struct {
    command: on_command,
    modal: on_modal,
}, struct {
    create: SubCommand {
        type: Integer {
            required: true,
            choices: [
                ("multiple-choice", PollType::MultipleChoice as i64),
                ("open-response", PollType::OpenResponse as i64),
                ("hybrid", PollType::Hybrid as i64),
                ("raffle", PollType::Raffle as i64),
            ],
        },
        duration: Integer {
            required: true,
            minimum: 1,
            maximum: 60 * 24 * 7,
        },
    },
    close: SubCommand {},
});

crate::define_commands! {
    self => {
        create => on_create_command;
        close => on_close_command;
    }
}

crate::define_modals! {
    create => on_create_modal;
}

/// Executes the create command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_create_command<'ap: 'ev, 'ev>(
    entry: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev CommandData>,
    resolver: CommandOptionResolver<'ev>,
) -> EventResult {
    let kind = resolver.integer("type")?;
    let duration = resolver.integer("duration")?;
    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let mut modal = ModalDataBuilder::new(
        entry.id("create")?.with_str(kind.to_string())?.with_str(duration.to_string())?,
        localize!(async(try in locale) category::UI, "poll-create-title").await?,
    )?;

    modal.input(
        TextInputBuilder::new(
            entry.id("title")?,
            localize!(async(try in locale) category::UI_INPUT, "poll-create-title").await?,
            TextInputStyle::Short,
        )?
        .min_length(1)?
        .max_length(256)?
        .required(true),
    )?;

    modal.input(
        TextInputBuilder::new(
            entry.id("image_url")?,
            localize!(async(try in locale) category::UI_INPUT, "poll-create-image").await?,
            TextInputStyle::Short,
        )?
        .required(false),
    )?;

    modal.input(
        TextInputBuilder::new(
            entry.id("description")?,
            localize!(async(try in locale) category::UI_INPUT, "poll-create-description").await?,
            TextInputStyle::Paragraph,
        )?
        .max_length(u16::try_from(DESCRIPTION_LENGTH / 2)?)?
        .required(false),
    )?;

    context.modal(modal.build()?).await?;

    crate::client::event::pass()
}

/// Handles a create modal.
///
/// # Errors
///
/// This function will return an error if the modal could not be handled.
async fn on_create_modal<'ap: 'ev, 'ev>(
    entry: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev ModalInteractionData>,
    custom_id: CustomId,
    resolver: ModalFieldResolver<'ev>,
) -> EventResult {
    let Some(guild_id) = context.interaction.guild_id else {
        bail!("this command must be used in a guild");
    };
    let Some(user) = context.interaction.author() else {
        bail!("this command must be used by a user");
    };
    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let kind = match custom_id.get::<i64>(0).transpose()? {
        Some(n) if n == PollType::MultipleChoice as i64 => PollType::MultipleChoice,
        Some(n) if n == PollType::OpenResponse as i64 => PollType::OpenResponse,
        Some(n) if n == PollType::Hybrid as i64 => PollType::Hybrid,
        Some(n) if n == PollType::Raffle as i64 => PollType::Raffle,
        Some(n) => bail!("invalid poll type: {n}"),
        None => bail!("missing poll type"),
    };
    let Some(duration) = custom_id.get::<NonZeroU16>(1).transpose()? else {
        bail!("missing poll duration");
    };

    let description = resolver.get(entry.id("description")?.to_string())?;
    let image_url = resolver.get(entry.id("image_url")?.to_string())?;

    let Some(title) = resolver.get(entry.id("title")?.to_string())? else {
        bail!("failed to resolve poll title");
    };

    if let Some(Err(error)) = image_url.map(ImageSource::url) {
        let error_title = localize!(async(try in locale) category::UI, "poll-invalid-url").await?;

        context.failure(error_title, Some(format!("> {error}"))).await?;

        return crate::client::event::pass();
    }

    let poll = Poll {
        user_id: user.id,
        guild_id,
        title: title.into(),
        about: description.map(Into::into),
        image: image_url.map(Into::into),
        kind,
        minutes: duration,
        state: PollState::Builder { inputs: Vec::new() },
    };

    poll.as_async_api().write().await?;

    let (embed, components) = poll.build(entry, locale, user, None).await?;

    crate::create_response!(context, struct {
        kind: InteractionResponseType::ChannelMessageWithSource,
        components: components,
        embeds: [embed],
        flags: MessageFlags::EPHEMERAL,
    })
    .await?;

    crate::client::event::pass()
}

/// Executes the close command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_close_command<'ap: 'ev, 'ev>(
    _: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev CommandData>,
    _resolver: CommandOptionResolver<'ev>,
) -> EventResult {
    context.defer(Visibility::Ephemeral).await?;

    let _locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    crate::client::event::pass()
}
