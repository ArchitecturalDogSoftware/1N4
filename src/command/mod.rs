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
use resolver::{CommandOptionResolver, ModalComponentResolver};
use twilight_model::application::command::{Command, CommandOptionChoice, CommandOptionType};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;

use self::context::Context;
use self::registry::CommandEntry;
use crate::client::event::EventResult;
use crate::define_command_modules;
use crate::utility::types::custom_id::CustomId;

/// Provides an interaction context API.
pub mod context;
/// Defines and implements the command registry.
pub mod registry;
/// Provides helpers for resolving command options.
pub mod resolver;

define_command_modules! {
    /// Provides all defined commands.
    pub mod definition {
        /// The echo command.
        pub mod echo;
        /// The help command.
        pub mod help;
        /// The localizer command.
        pub mod localizer;
        /// The ping command.
        pub mod ping;
        /// The role command.
        pub mod role;
    }
}

/// A type that can be invoked to construct a command.
#[async_trait::async_trait]
pub trait CommandFactory: Send + Sync {
    /// Creates an API command value.
    ///
    /// # Errors
    ///
    /// This function will return an error if command creation fails.
    async fn build(&self, entry: &CommandEntry, guild_id: Option<Id<GuildMarker>>) -> Result<Option<Command>>;
}

/// A type that can be invoked to execute a command.
#[async_trait::async_trait]
pub trait CommandCallable: Send + Sync {
    /// Executes a command.
    ///
    /// # Errors
    ///
    /// This function will return an error if execution fails.
    async fn on_command<'ap: 'ev, 'ev>(
        &self,
        entry: &CommandEntry,
        context: Context<'ap, 'ev, &'ev CommandData>,
        resolver: CommandOptionResolver<'ev>,
    ) -> EventResult;
}

/// A type that can be invoked to execute a component.
#[async_trait::async_trait]
pub trait ComponentCallable: Send + Sync {
    /// Executes a component.
    ///
    /// # Errors
    ///
    /// This function will return an error if execution fails.
    async fn on_component<'ap: 'ev, 'ev>(
        &self,
        entry: &CommandEntry,
        context: Context<'ap, 'ev, &'ev MessageComponentInteractionData>,
        custom_id: CustomId,
    ) -> EventResult;
}

/// A type that can be invoked to execute a modal.
#[async_trait::async_trait]
pub trait ModalCallable: Send + Sync {
    /// Executes a modal.
    ///
    /// # Errors
    ///
    /// This function will return an error if execution fails.
    async fn on_modal<'ap: 'ev, 'ev>(
        &self,
        entry: &CommandEntry,
        context: Context<'ap, 'ev, &'ev ModalInteractionData>,
        custom_id: CustomId,
        resolver: ModalComponentResolver<'ev>,
    ) -> EventResult;
}

/// A type that can be invoked to execute an auto-completion.
#[async_trait::async_trait]
pub trait AutocompleteCallable: Send + Sync {
    /// Executes an auto-completion.
    ///
    /// # Errors
    ///
    /// This function will return an error if execution fails.
    async fn on_autocomplete<'ap: 'ev, 'ev>(
        &self,
        entry: &CommandEntry,
        context: Context<'ap, 'ev, &'ev CommandData>,
        resolver: CommandOptionResolver<'ev>,
        option: &'ev str,
        current: &'ev str,
        kind: CommandOptionType,
    ) -> Result<Box<[CommandOptionChoice]>>;
}

/// Defines and matches for commands.
///
/// # Examples
///
/// ```
/// crate::define_commands! {
///     self => {
///         invoke => on_invoke_command;
///     }
///
///     group => {
///         invoke => on_group_invoke_command;
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_commands {
    (
        self => {
            $($name:ident => $callback:ident;)*
        }
        $($group:ident => {
            $($group_name:ident => $group_callback:ident;)*
        })*
    ) => {
        /// Defines the command's command callbacks.
        mod command {
            $(pub mod $name {
                pub(in super::super) use super::super::$callback as callback;

                /// The command's name.
                pub const NAME: &::std::primitive::str = ::std::stringify!($name);
            })*

            $(pub mod $group {
                /// The group's name.
                pub const NAME: &::std::primitive::str = ::std::stringify!($group);

                $(pub mod $group_name {
                    pub(in super::super::super) use super::super::super::$group_callback as callback;

                    /// The command's name.
                    pub const NAME: &::std::primitive::str = ::std::stringify!($group_name);
                })*
            })*
        }

        /// Executes the command.
        ///
        /// # Errors
        ///
        /// This function will return an error if the command could not be executed.
        async fn on_command<'ap: 'ev, 'ev>(
            entry: &$crate::command::registry::CommandEntry,
            context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::application_command::CommandData>,
            resolver: $crate::command::resolver::CommandOptionResolver<'ev>,
        ) -> $crate::client::event::EventResult {
            $(if let Ok(resolver) = resolver.subcommand(self::command::$name::NAME) {
                return self::command::$name::callback(entry, context, resolver).await;
            })else*

            $(if let Ok(resolver) = resolver.subcommand_group(self::command::$group::NAME) {
                $(if let Ok(resolver) = resolver.subcommand(self::command::$group::$group_name::NAME) {
                    return self::command::$group::$group_name::callback(entry, context, resolver).await;
                })*
            })else*

            ::anyhow::bail!("unknown or missing subcommand")
        }
    };
}

/// Defines and matches for components.
///
/// # Examples
///
/// ```
/// crate::define_components! {
///     select => on_select_component,
/// }
/// ```
#[macro_export]
macro_rules! define_components {
    ($($name:ident => $call:ident;)*) => {
        /// Defines the command's component callbacks.
        mod component {$(
            pub mod $name {
                pub(in super::super) use super::super::$call as callback;

                /// The component's name.
                pub const NAME: &::std::primitive::str = ::std::stringify!($name);
            }
        )*}

        /// Executes the component.
        ///
        /// # Errors
        ///
        /// This function will return an error if the component could not be executed.
        async fn on_component<'ap: 'ev, 'ev>(
            entry: &$crate::command::registry::CommandEntry,
            context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::message_component::MessageComponentInteractionData>,
            custom_id: $crate::utility::types::custom_id::CustomId,
        ) -> $crate::client::event::EventResult {
            match &**custom_id.variant() {
                $(self::component::$name::NAME => self::component::$name::callback(entry, context, custom_id).await,)*
                _ => ::anyhow::bail!("unknown or missing component"),
            }
        }
    };
}

/// Defines and matches for modals.
///
/// # Examples
///
/// ```
/// crate::define_modals! {
///     select => on_select_modal,
/// }
/// ```
#[macro_export]
macro_rules! define_modals {
    ($($name:ident => $call:ident;)*) => {
        /// Defines the command's modal callbacks.
        mod modal {$(
            pub mod $name {
                pub(in super::super) use super::super::$call as callback;

                /// The modal's name.
                pub const NAME: &::std::primitive::str = ::std::stringify!($name);
            }
        )*}

        /// Executes the modal.
        ///
        /// # Errors
        ///
        /// This function will return an error if the modal could not be executed.
        async fn on_modal<'ap: 'ev, 'ev>(
            entry: &$crate::command::registry::CommandEntry,
            context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::modal::ModalInteractionData>,
            custom_id: $crate::utility::types::custom_id::CustomId,
            resolver: $crate::command::resolver::ModalComponentResolver<'ev>,
        ) -> $crate::client::event::EventResult {
            match &**custom_id.variant() {
                $(self::modal::$name::NAME => self::modal::$name::callback(entry, context, custom_id, resolver).await,)*
                _ => ::anyhow::bail!("unknown or missing modal"),
            }
        }
    };
}
