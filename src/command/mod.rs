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
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::LazyLock;

use anyhow::{ensure, Result};
use context::Context;
use ina_logging::info;
use tokio::sync::RwLock;
use twilight_model::application::command::Command;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

/// Provides an interaction context API.
pub mod context;

/// The command registry instance.
static REGISTRY: LazyLock<RwLock<CommandRegistry>> = LazyLock::new(RwLock::default);

/// A command entry creation function.
type CreateFunction = fn(&CommandEntry, Option<Id<GuildMarker>>) -> Result<Option<Command>>;
/// A command entry callback function.
type Callback<T> = for<'ap, 'ev> fn(Context<'ap, 'ev, T>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// The command registry.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandRegistry {
    /// The inner command list.
    inner: HashMap<&'static str, CommandEntry>,
}

impl CommandRegistry {
    /// Creates a new [`CommandRegistry`].
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { inner: HashMap::new() }
    }

    /// Returns whether this [`CommandRegistry`] contains a command with the given name.
    #[inline]
    #[must_use]
    pub fn contains(&self, name: &'static str) -> bool {
        self.inner.contains_key(name)
    }

    /// Returns the command assigned to the given name from within this [`CommandRegistry`].
    #[inline]
    #[must_use]
    pub fn command(&self, name: &'static str) -> Option<&CommandEntry> {
        self.inner.get(name)
    }

    /// Returns an iterator over references to the entries within this [`CommandRegistry`].
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &CommandEntry> {
        self.inner.values()
    }

    /// Returns an iterator over mutable references to the entries within this [`CommandRegistry`].
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut CommandEntry> {
        self.inner.values_mut()
    }

    /// Registers the given command entry.
    ///
    /// # Errors
    ///
    /// This function will return an error if a command with the same name was already registered.
    pub fn register(&mut self, command: CommandEntry) -> Result<()> {
        ensure!(!self.contains(command.name), "command '{}' is already registered", command.name);

        self.inner.insert(command.name, command);

        Ok(())
    }

    /// Builds and returns a list of all registered commands.
    ///
    /// # Errors
    ///
    /// This function will return an error if a command fails to build.
    pub fn collect<T>(&self, guild_id: Option<Id<GuildMarker>>) -> Result<T>
    where
        T: FromIterator<Command>,
    {
        self.iter().filter_map(|entry| (entry.create)(entry, guild_id).transpose()).collect()
    }
}

impl IntoIterator for CommandRegistry {
    type IntoIter = std::collections::hash_map::IntoValues<&'static str, Self::Item>;
    type Item = CommandEntry;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_values()
    }
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

/// Returns a reference to the command registry.
pub async fn registry() -> impl Deref<Target = CommandRegistry> {
    REGISTRY.read().await
}

/// Returns a mutable reference to the command registry.
pub async fn registry_mut() -> impl DerefMut<Target = CommandRegistry> {
    REGISTRY.write().await
}

/// Initializes the command registry.
///
/// # Errors
///
/// This function will return an error if a command fails to be registered.
pub async fn initialize() -> Result<()> {
    let registry = self::registry_mut().await;

    // TODO: Implement commands and register them here.

    drop(registry);

    info!(async "initialized command registry").await.map_err(Into::into)
}
