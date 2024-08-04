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

use anyhow::Result;
use twilight_model::application::command::{CommandOptionChoice, CommandOptionType, CommandType};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::guild::Permissions;

use crate::command::context::Context;
use crate::utility::traits::convert::AsLocale;
use crate::utility::types::id::CustomId;

crate::define_command!("help", CommandType::ChatInput, struct {
    allow_dms: true,
    permissions: Permissions::USE_SLASH_COMMANDS,
}, struct {
    command_callback: on_command,
    component_callback: on_component,
    modal_callback: on_modal,
    autocomplete_callback: on_autocomplete,
}, struct {});

async fn on_command<'ap: 'ev, 'ev>(mut context: Context<'ap, 'ev, &'ev CommandData>) -> Result<bool> {
    context.defer(true).await?;

    let locale = context.interaction.author().map(AsLocale::as_locale).transpose()?;

    Ok(false)
}

async fn on_component<'ap: 'ev, 'ev>(
    _context: Context<'ap, 'ev, &'ev MessageComponentInteractionData>,
    _custom_id: CustomId,
) -> Result<bool> {
    Ok(false)
}

async fn on_modal<'ap: 'ev, 'ev>(
    _context: Context<'ap, 'ev, &'ev ModalInteractionData>,
    _custom_id: CustomId,
) -> Result<bool> {
    Ok(false)
}

async fn on_autocomplete<'ap: 'ev, 'ev>(
    _context: Context<'ap, 'ev, &'ev CommandData>,
    _option: &'ev str,
    _current: &'ev str,
    _kind: CommandOptionType,
) -> Result<Box<[CommandOptionChoice]>> {
    Ok(Box::new([]))
}
