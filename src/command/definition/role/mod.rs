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

use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::application_command::CommandData;

use crate::client::event::EventResult;
use crate::command::context::Context;
use crate::command::registry::CommandEntry;

/// The command's data.
mod data;

pub const SELECT_COMPONENT_NAME: &str = "select";

crate::define_command!("role", CommandType::ChatInput, struct {
    allow_dms: true,
}, struct {
    command: on_command,
}, struct {
    create: SubCommand {
        role: Role {
            required: true,
        },
        icon: String {
            required: true,
        },
    },
    delete: SubCommand {
        role: Role {
            required: true,
        }
    },
    preview: SubCommand {},
    finish: SubCommand {},
});

/// Executes the command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_command<'ap: 'ev, 'ev>(
    entry: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev CommandData>,
) -> EventResult {
    todo!()
}
