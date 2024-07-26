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

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use context::Context;
use twilight_model::application::command::Command;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

/// Provides an interaction context API.
pub mod context;

/// A command entry creation function.
type CreateFunction = fn(&CommandEntry, Option<Id<GuildMarker>>) -> Result<Option<Command>>;
/// A command entry callback function.
type Callback<T> = for<'ap, 'ev> fn(Context<'ap, 'ev, T>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// The command registry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandRegistry {
    /// The inner command list.
    inner: HashMap<&'static str, CommandEntry>,
}

/// An entry within the command registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandEntry {
    /// The command's literal name.
    pub name: &'static str,
    /// The command's creation function.
    pub create: CreateFunction,
    /// The command's callback functions.
    pub callbacks: CommandEntryCallbacks,
}

/// The callback functions of a [`CommandEntry`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommandEntryCallbacks {
    /// The command callback.
    pub command: Option<Callback<CommandData>>,
    /// The component callback.
    pub component: Option<Callback<MessageComponentInteractionData>>,
    /// The modal callback.
    pub modal: Option<Callback<ModalInteractionData>>,
    /// The autocompletion callback.
    pub autocomplete: Option<Callback<CommandData>>,
}
