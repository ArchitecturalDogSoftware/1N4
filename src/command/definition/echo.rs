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

use anyhow::{bail, Result};
use ina_localization::localize;
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::guild::Permissions;

use crate::command::context::Context;
use crate::command::resolver::CommandOptionResolver;
use crate::utility::category;
use crate::utility::traits::convert::AsLocale;

crate::define_command!("echo", CommandType::ChatInput, struct {
    allow_dms: true,
    permissions: Permissions::ADMINISTRATOR,
}, struct {
    command_callback: on_command,
}, struct {
    content: String {
        required: true,
        maximum: 2000,
    }
});

/// Executes the command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_command<'ap: 'ev, 'ev>(mut context: Context<'ap, 'ev, &'ev CommandData>) -> Result<bool> {
    let resolver = CommandOptionResolver::new(context.state);
    let Some(ref channel) = context.interaction.channel else { bail!("this command must be used in a channel") };

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localization::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    context.api.client.create_message(channel.id).content(resolver.get_str("content")?).await?;
    context.text(localize!(async(try in locale) category::UI, "echo-done").await?, true).await?;

    Ok(false)
}
