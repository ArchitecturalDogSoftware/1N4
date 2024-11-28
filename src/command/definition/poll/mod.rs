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

use std::num::NonZeroU16;

use anyhow::bail;
use data::poll::{PollBuilder, PollType};
use ina_localizing::localize;
use ina_storage::stored::Stored;
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::channel::message::component::TextInputStyle;
use twilight_model::guild::Permissions;
use twilight_util::builder::embed::ImageSource;

use crate::client::event::EventResult;
use crate::command::context::{Context, Visibility};
use crate::command::registry::CommandEntry;
use crate::command::resolver::{CommandOptionResolver, ModalFieldResolver};
use crate::utility::category;
use crate::utility::traits::convert::AsLocale;
use crate::utility::types::builder::TextInputBuilder;
use crate::utility::types::id::CustomId;
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
    contexts: [InteractionContextType::Guild],
    permissions: Permissions::SEND_POLLS,
}, struct {
    command: on_command,
    component: on_component,
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

crate::define_components! {}

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
        CustomId::<Box<str>>::new(entry.name, "create")?
            .with(kind.to_string())?
            .with(duration.to_string())?
            .to_string(),
        localize!(async(try in locale) category::UI, "poll-create-title").await?.to_string(),
    )?;

    modal.input(
        TextInputBuilder::new(
            CustomId::<Box<str>>::new(entry.name, "title")?.to_string(),
            localize!(async(try in locale) category::UI_INPUT, "poll-create-title").await?.to_string(),
            TextInputStyle::Short,
        )?
        .min_length(1)?
        .max_length(256)?
        .required(true),
    )?;

    modal.input(
        TextInputBuilder::new(
            CustomId::<Box<str>>::new(entry.name, "image_url")?.to_string(),
            localize!(async(try in locale) category::UI_INPUT, "poll-create-image").await?.to_string(),
            TextInputStyle::Short,
        )?
        .required(false),
    )?;

    modal.input(
        TextInputBuilder::new(
            CustomId::<Box<str>>::new(entry.name, "description")?.to_string(),
            localize!(async(try in locale) category::UI_INPUT, "poll-create-description").await?.to_string(),
            TextInputStyle::Paragraph,
        )?
        .max_length(4096)?
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
    event: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev ModalInteractionData>,
    custom_id: CustomId,
) -> EventResult {
    let Some(guild_id) = context.interaction.guild_id else {
        bail!("this command must be used in a guild");
    };
    let Some(user_id) = context.interaction.author_id() else {
        bail!("this command must be used by a user");
    };
    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let kind = match custom_id.data().first().map(|s| s.parse::<i64>()).transpose()? {
        Some(n) if n == PollType::MultipleChoice as i64 => PollType::MultipleChoice,
        Some(n) if n == PollType::OpenResponse as i64 => PollType::OpenResponse,
        Some(n) if n == PollType::Hybrid as i64 => PollType::Hybrid,
        Some(n) if n == PollType::Raffle as i64 => PollType::Raffle,
        Some(n) => bail!("invalid poll type: {n}"),
        None => bail!("missing poll type"),
    };
    let Some(duration) = custom_id.data().get(1).map(|s| s.parse::<NonZeroU16>()).transpose()? else {
        bail!("missing poll duration")
    };

    let resolver = ModalFieldResolver::new(context.data);
    let image_url = resolver.get(CustomId::<Box<str>>::new(event.name, "image_url")?.to_string())?;
    let description = resolver.get(CustomId::<Box<str>>::new(event.name, "description")?.to_string())?;
    let Some(title) = resolver.get(CustomId::<Box<str>>::new(event.name, "title")?.to_string())? else {
        bail!("failed to resolve poll title");
    };

    if let Some(Err(error)) = image_url.map(ImageSource::url) {
        let error_title = localize!(async(try in locale) category::UI, "poll-invalid-url").await?;

        context.failure(error_title, Some(format!("> {error}"))).await?;

        return crate::client::event::pass();
    }

    let poll = PollBuilder {
        user_id,
        guild_id,
        kind,
        title: title.into(),
        description: description.map(Into::into),
        image_url: image_url.map(Into::into),
        duration,
        inputs: vec![],
    };

    poll.as_async_api().write().await?;

    context.embed(poll.build_preview(locale).await?, Visibility::Ephemeral).await?;

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
