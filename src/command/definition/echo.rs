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

use anyhow::bail;
use ina_localization::localize;
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::guild::Permissions;

use crate::client::event::EventResult;
use crate::command::context::Context;
use crate::command::registry::CommandEntry;
use crate::command::resolver::CommandOptionResolver;
use crate::utility::category;
use crate::utility::traits::convert::AsLocale;

crate::define_command!("echo", CommandType::ChatInput, struct {
    allow_dms: true,
    permissions: Permissions::ADMINISTRATOR,
}, struct {
    command: on_command,
}, struct {
    content: String {
        required: true,
        // This can blow up fast when formatting.
        maximum: 500,
    },
    format: Integer {
        choices: [("plain", 0), ("binary", 1), ("octal", 2), ("decimal", 3), ("hexadecimal", 4)],
    }
});

/// Executes the command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_command<'ap: 'ev, 'ev>(_: &CommandEntry, mut context: Context<'ap, 'ev, &'ev CommandData>) -> EventResult {
    let Some(ref channel) = context.interaction.channel else {
        bail!("this command must be used in a channel");
    };

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localization::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let resolver = CommandOptionResolver::new(context.state);
    let message = resolver.get_str("content")?;
    let message: Box<[_]> = match resolver.get_i64("format").copied().unwrap_or(0) {
        1 => message.chars().map(|c| format!("0b{:b}", u32::from(c))).collect(),
        2 => message.chars().map(|c| format!("0o{:o}", u32::from(c))).collect(),
        3 => message.chars().map(|c| format!("0d{}", u32::from(c))).collect(),
        4 => message.chars().map(|c| format!("0x{:X}", u32::from(c))).collect(),
        _ => Box::new([message.to_string()]),
    };

    context.api.client.create_message(channel.id).content(&message.join(" ")).await?;
    context.text(localize!(async(try in locale) category::UI, "echo-done").await?, true).await?;

    crate::client::event::pass()
}
