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

use ina_localizing::localize;
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_util::builder::embed::EmbedBuilder;

use crate::client::event::EventResult;
use crate::command::context::Context;
use crate::command::registry::CommandEntry;
use crate::utility::traits::convert::AsLocale;
use crate::utility::traits::extension::IdExt;
use crate::utility::{category, color};

crate::define_entry!("ping", CommandType::ChatInput, struct {
    allow_dms: true,
}, struct {
    command: on_command,
}, struct {});

/// Executes the command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_command<'ap: 'ev, 'ev>(_: &CommandEntry, mut context: Context<'ap, 'ev, &'ev CommandData>) -> EventResult {
    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let title = localize!(async(try in locale) category::UI, "ping-start").await?;
    let embed = EmbedBuilder::new().title(title).color(color::BRANDING_B);

    context.embed(embed.build(), true).await?;

    let response = context.client().response(&context.interaction.token).await?.model().await?;
    let delay = response.id.creation_date() - context.interaction.id.creation_date();
    let title = localize!(async(try in locale) category::UI, "ping-finish").await?;
    let embed = EmbedBuilder::new().title(format!("{title} ({delay})")).color(color::BRANDING_A);

    context.client().update_response(&context.interaction.token).embeds(Some(&[embed.build()])).await?;

    crate::client::event::pass()
}
