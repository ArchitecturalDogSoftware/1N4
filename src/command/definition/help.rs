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
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::guild::Permissions;

use crate::command::context::Context;

crate::define_command!("help", CommandType::ChatInput, struct {
    dev_only: true,
    allow_dms: true,
    permissions: Permissions::USE_SLASH_COMMANDS,
}, struct {
    command_callback: _on_command,
}, struct {});

async fn _on_command<'ap: 'ev, 'ev>(_context: Context<'ap, 'ev, &'ev CommandData>) -> Result<bool> {
    ina_logging::debug!(async "test async call").await?;

    Ok(false)
}
