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
use std::ops::{Deref, DerefMut};
use std::sync::LazyLock;

use anyhow::{ensure, Result};
use ina_logging::info;
use tokio::sync::RwLock;
use twilight_model::application::command::Command;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

use super::{AutocompleteCallable, CommandCallable, CommandFactory, ComponentCallable, ModalCallable};

/// The command registry instance.
static REGISTRY: LazyLock<RwLock<CommandRegistry>> = LazyLock::new(RwLock::default);

/// The command registry.
#[repr(transparent)]
#[derive(Default)]
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
    pub fn contains(&self, name: &str) -> bool {
        self.inner.contains_key(name)
    }

    /// Returns the command assigned to the given name from within this [`CommandRegistry`].
    #[inline]
    #[must_use]
    pub fn command(&self, name: &str) -> Option<&CommandEntry> {
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
    pub async fn collect<T>(&self, guild_id: Option<Id<GuildMarker>>) -> Result<T>
    where
        T: FromIterator<Command>,
    {
        let mut buffer = Vec::with_capacity(self.inner.len());

        for entry in self.iter() {
            let command = entry.factory.build(entry, guild_id).await;
            let Some(command) = command.transpose() else { continue };

            buffer.push(command?);
        }

        Ok(buffer.into_iter().collect())
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
pub struct CommandEntry {
    /// The command's literal name.
    pub name: &'static str,
    /// The command's creation function.
    pub factory: Box<dyn CommandFactory>,
    /// The command's callback functions.
    pub callbacks: CommandEntryCallbacks,
}

/// The callback functions of a [`CommandEntry`].
#[derive(Default)]
pub struct CommandEntryCallbacks {
    /// The command callback.
    pub command: Option<Box<dyn CommandCallable>>,
    /// The component callback.
    pub component: Option<Box<dyn ComponentCallable>>,
    /// The modal callback.
    pub modal: Option<Box<dyn ModalCallable>>,
    /// The autocompletion callback.
    pub autocomplete: Option<Box<dyn AutocompleteCallable>>,
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
    let mut registry = self::registry_mut().await;

    registry.register(super::definition::echo::entry())?;
    registry.register(super::definition::help::entry())?;
    registry.register(super::definition::localizer::entry())?;
    registry.register(super::definition::ping::entry())?;

    drop(registry);

    info!(async "initialized command registry").await.map_err(Into::into)
}
