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
use context::Context;
use ina_logging::info;
use tokio::sync::RwLock;
use twilight_model::application::command::{Command, CommandOptionChoice, CommandOptionType};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

use crate::utility::types::id::CustomId;

/// Provides an interaction context API.
pub mod context;
/// Provides helpers for resolving command options.
pub mod resolver;
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
}

/// The command registry instance.
static REGISTRY: LazyLock<RwLock<CommandRegistry>> = LazyLock::new(RwLock::default);

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
    async fn on_command<'ap: 'ev, 'ev>(&self, context: Context<'ap, 'ev, &'ev CommandData>) -> Result<bool>;
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
        context: Context<'ap, 'ev, &'ev MessageComponentInteractionData>,
        custom_id: CustomId,
    ) -> Result<bool>;
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
        context: Context<'ap, 'ev, &'ev ModalInteractionData>,
        custom_id: CustomId,
    ) -> Result<bool>;
}

/// A type that can be invoked to execute an autocompletion.
#[async_trait::async_trait]
pub trait AutocompleteCallable: Send + Sync {
    /// Executes an autocompletion.
    ///
    /// # Errors
    ///
    /// This function will return an error if execution fails.
    async fn on_autocomplete<'ap: 'ev, 'ev>(
        &self,
        context: Context<'ap, 'ev, &'ev CommandData>,
        option: &'ev str,
        current: &'ev str,
        kind: CommandOptionType,
    ) -> Result<Box<[CommandOptionChoice]>>;
}

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

    registry.register(self::definition::echo::entry())?;
    registry.register(self::definition::help::entry())?;
    registry.register(self::definition::localizer::entry())?;
    registry.register(self::definition::ping::entry())?;

    drop(registry);

    info!(async "initialized command registry").await.map_err(Into::into)
}

/// Recursively attempts to find a focused option within the given iterator.
pub fn find_focused_option<'cd, I>(options: I) -> Option<(&'cd str, &'cd str, CommandOptionType)>
where
    I: IntoIterator<Item = &'cd CommandDataOption>,
{
    options.into_iter().find_map(|option| match &option.value {
        CommandOptionValue::Focused(text, kind) => Some((&(*option.name), &(**text), *kind)),
        CommandOptionValue::SubCommand(options) => self::find_focused_option(options),
        CommandOptionValue::SubCommandGroup(commands) => self::find_focused_option(commands),
        _ => None,
    })
}

/// Creates a command.
///
/// # Examples
///
/// ```
/// define_command!("help", CommandType::ChatInput, struct {
///     dev_only: false,
///     allow_dms: true,
///     is_nsfw: false,
///     permissions: Permissions::USE_SLASH_COMMANDS,
/// }, struct {
///     command_callback: command,
/// }, struct {
///     ephemeral: Boolean {
///         required: true,
///     }
/// });
/// ```
#[macro_export]
macro_rules! define_command {
    (
        $name:literal, $type:expr, struct {
            $(dev_only: $dev_only:literal,)?
            $(allow_dms: $allow_dms:literal,)?
            $(is_nsfw: $is_nsfw:literal,)?
            $(permissions: $permissions:expr,)?
        },struct {
            $(command_callback: $command_callback:expr,)?
            $(component_callback: $component_callback:expr,)?
            $(modal_callback: $modal_callback:expr,)?
            $(autocomplete_callback: $autocomplete_callback:expr,)?
        },struct { $($option_name:ident : $option_kind:ident { $($body:tt)* }),* $(,)? }
    ) => {
        /// The command implementation.
        pub struct Impl;

        #[::async_trait::async_trait]
        impl $crate::command::CommandFactory for Impl {
            async fn build(
                &self,
                entry: &$crate::command::CommandEntry,
                guild_id: ::std::option::Option<::twilight_model::id::Id<::twilight_model::id::marker::GuildMarker>>,
            ) -> ::anyhow::Result<::std::option::Option<::twilight_model::application::command::Command>>
            {
                $(if $dev_only && guild_id.is_none() {
                    return ::std::result::Result::Ok(::std::option::Option::None);
                })?

                let localizer_name_key = ::std::format!("{}-name", entry.name);
                let localizer_description_key = ::std::format!("{}-description", entry.name);

                let localized_name = ::ina_localization::localize!(async $crate::utility::category::COMMAND, &(*localizer_description_key)).await?;
                let mut builder = ::twilight_util::builder::command::CommandBuilder::new(entry.name, localized_name, $type);

                $(builder = builder.dm_permission($allow_dms);)?
                $(builder = builder.nsfw($is_nsfw);)?
                $(builder = builder.default_member_permissions($permissions);)?

                if let ::std::option::Option::Some(guild_id) = guild_id {
                    builder = builder.guild_id(guild_id);
                }

                let locales = ::ina_localization::thread::list().await?;
                let mut localized_names = ::std::vec::Vec::with_capacity(locales.len());
                let mut localized_descriptions = ::std::vec::Vec::with_capacity(locales.len());

                for locale in &locales {
                    let name = <_ as ::std::string::ToString>::to_string(&::ina_localization::localize!(async(in *locale) $crate::utility::category::COMMAND, &(*localizer_name_key)).await?);
                    let description = <_ as ::std::string::ToString>::to_string(&::ina_localization::localize!(async(in *locale) $crate::utility::category::COMMAND, &(*localizer_description_key)).await?);

                    localized_names.push((<_ as ::std::string::ToString>::to_string(locale), name));
                    localized_descriptions.push((<_ as ::std::string::ToString>::to_string(locale), description));
                }

                builder = builder.name_localizations(localized_names);
                builder = builder.description_localizations(localized_descriptions);

                $(builder = builder.option($crate::define_command!(@option(entry, $option_name, $option_kind, &locales, { $($body)* })));)*

                ::std::result::Result::Ok(::std::option::Option::Some(builder.validate()?.build()))
            }
        }

        $(
            #[::async_trait::async_trait]
            impl $crate::command::CommandCallable for Impl {
                #[inline]
                async fn on_command<'ap: 'ev, 'ev>(
                    &self,
                    context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::application_command::CommandData>,
                ) -> ::anyhow::Result<::std::primitive::bool>
                {
                    $command_callback(context).await
                }
            }
        )?

        $(
            #[::async_trait::async_trait]
            impl $crate::command::ComponentCallable for Impl {
                #[inline]
                async fn on_component<'ap: 'ev, 'ev>(
                    &self,
                    context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::message_component::MessageComponentInteractionData>,
                    custom_id: $crate::utility::types::id::CustomId,
                ) -> ::anyhow::Result<::std::primitive::bool>
                {
                    $component_callback(context, custom_id).await
                }
            }
        )?

        $(
            #[::async_trait::async_trait]
            impl $crate::command::ModalCallable for Impl {
                #[inline]
                async fn on_modal<'ap: 'ev, 'ev>(
                    &self,
                    context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::modal::ModalInteractionData>,
                    custom_id: $crate::utility::types::id::CustomId,
                ) -> ::anyhow::Result<::std::primitive::bool>
                {
                    $modal_callback(context, custom_id).await
                }
            }
        )?

        $(
            #[::async_trait::async_trait]
            impl $crate::command::AutocompleteCallable for Impl {
                #[inline]
                async fn on_autocomplete<'ap: 'ev, 'ev>(
                    &self,
                    context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::application_command::CommandData>,
                    option: &'ev ::std::primitive::str,
                    current: &'ev ::std::primitive::str,
                    kind: ::twilight_model::application::command::CommandOptionType,
                ) -> ::anyhow::Result<::std::boxed::Box<[::twilight_model::application::command::CommandOptionChoice]>>
                {
                    $autocomplete_callback(context, option, current, kind).await
                }
            }
        )?

        /// Returns this command's registry entry.
        #[must_use = r"command entries should be registered"]
        pub fn entry() -> $crate::command::CommandEntry {
            #[allow(unused_mut)]
            let mut entry = $crate::command::CommandEntry {
                name: $name,
                factory: ::std::boxed::Box::new(Impl),
                callbacks: <$crate::command::CommandEntryCallbacks as ::std::default::Default>::default(),
            };

            $({
                let _ = $command_callback;

                entry.callbacks.command = ::std::option::Option::Some(::std::boxed::Box::new(Impl));
            })?
            $({
                let _ = $component_callback;

                entry.callbacks.component = ::std::option::Option::Some(::std::boxed::Box::new(Impl));
            })?
            $({
                let _ = $modal_callback;

                entry.callbacks.modal = ::std::option::Option::Some(::std::boxed::Box::new(Impl));
            })?
            $({
                let _ = $autocomplete_callback;

                entry.callbacks.autocomplete = ::std::option::Option::Some(::std::boxed::Box::new(Impl));
            })?

            entry
        }
    };
    (@option($entry:expr, $name:ident, $kind:ident, $locales:expr, {
        $($body:tt)*
    })) => {{
        let name = ::std::stringify!($name);
        let localizer_name_key = ::std::format!("{}-{name}-name", $entry.name);
        let localizer_description_key = ::std::format!("{}-{name}-description", $entry.name);

        let mut builder = $crate::define_command!(@option<$kind>($entry, name, &localizer_name_key, $locales, { $($body)* }));

        let mut localized_names = ::std::vec::Vec::with_capacity($locales.len());
        let mut localized_descriptions = ::std::vec::Vec::with_capacity($locales.len());

        for locale in $locales {
            let name = <_ as ::std::string::ToString>::to_string(&::ina_localization::localize!(async(in *locale) $crate::utility::category::COMMAND_OPTION, &(*localizer_name_key)).await?);
            let description = <_ as ::std::string::ToString>::to_string(&::ina_localization::localize!(async(in *locale) $crate::utility::category::COMMAND_OPTION, &(*localizer_description_key)).await?);

            localized_names.push((<_ as ::std::string::ToString>::to_string(locale), name));
            localized_descriptions.push((<_ as ::std::string::ToString>::to_string(locale), description));
        }

        builder = builder.name_localizations(localized_names);
        builder = builder.description_localizations(localized_descriptions);

        builder
    }};
    (@option<Attachment>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
    })) => {{
        ::twilight_util::builder::command::AttachmentBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
    }};
    (@option<Boolean>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
    })) => {{
        ::twilight_util::builder::command::BooleanBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
    }};
    (@option<Channel>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
        $(channel_types: $channel_types:expr,)?
    })) => {{
        ::twilight_util::builder::command::ChannelBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
        $(.channel_types($channel_types))?
    }};
    (@option<Integer>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
        $(autocomplete: $autocomplete:expr,)?
        $(minimum: $minimum:expr,)?
        $(maximum: $maximum:expr,)?
        $(choices: [$(($choice_name:expr, $choice_value:expr)),+ $(,)?],)?
    })) => {{
        ::twilight_util::builder::command::IntegerBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
        $(.autocomplete($autocomplete))?
        $(.min_value($minimum))?
        $(.max_value($maximum))?
        $(
            .choices([$(($choice_name, $choice_value)),*])
            $(.choice_localizations($choice_name, {
                let localizer_key = ::std::format!("{}-{}-{}", $entry.name, $name, $choice_name);
                let mut localized = ::std::vec::Vec::with_capacity($locales.len());

                for locale in $locales {
                    let name = <_ as ::std::string::ToString>::to_string(&::ina_localization::localize!(async(in *locale) $crate::utility::category::COMMAND_CHOICE, &(*localizer_key)).await?);

                    localized.push((<_ as ::std::string::ToString>::to_string(locale), name));
                }

                localized
            }))*
        )?
    }};
    (@option<Mentionable>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
    })) => {{
        ::twilight_util::builder::command::MentionableBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
    }};
    (@option<Number>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
        $(autocomplete: $autocomplete:expr,)?
        $(minimum: $minimum:expr,)?
        $(maximum: $maximum:expr,)?
        $(choices: [$(($choice_name:expr, $choice_value:expr)),+ $(,)?],)?
    })) => {{
        ::twilight_util::builder::command::NumberBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
        $(.autocomplete($autocomplete))?
        $(.min_value($minimum))?
        $(.max_value($maximum))?
        $(
            .choices([$(($choice_name, $choice_value)),*])
            $(.choice_localizations($choice_name, {
                let localizer_key = ::std::format!("{}-{}-{}", $entry.name, $name, $choice_name);
                let mut localized = ::std::vec::Vec::with_capacity($locales.len());

                for locale in $locales {
                    let name = <_ as ::std::string::ToString>::to_string(&::ina_localization::localize!(async(in *locale) "choice", &(*localizer_key)).await?);

                    localized.push((<_ as ::std::string::ToString>::to_string(locale), name));
                }

                localized
            }))*
        )?
    }};
    (@option<Role>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
    })) => {{
        ::twilight_util::builder::command::RoleBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
    }};
    (@option<String>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
        $(autocomplete: $autocomplete:expr,)?
        $(minimum: $minimum:expr,)?
        $(maximum: $maximum:expr,)?
        $(choices: [$(($choice_name:expr, $choice_value:expr)),+ $(,)?],)?
    })) => {{
        ::twilight_util::builder::command::StringBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
        $(.autocomplete($autocomplete))?
        $(.min_length($minimum))?
        $(.max_length($maximum))?
        $(
            .choices([$(($choice_name, $choice_value)),*])
            $(.choice_localizations($choice_name, {
                let localizer_key = ::std::format!("{}-{}-{}", $entry.name, $name, $choice_name);
                let mut localized = ::std::vec::Vec::with_capacity($locales.len());

                for locale in $locales {
                    let name = <_ as ::std::string::ToString>::to_string(&::ina_localization::localize!(async(in *locale) "choice", &(*localizer_key)).await?);

                    localized.push((<_ as ::std::string::ToString>::to_string(locale), name));
                }

                localized
            }))*
        )?
    }};
    (@option<SubCommand>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $($option_name:ident : $option_kind:ident { $($body:tt)* }),* $(,)?
    })) => {{
        ::twilight_util::builder::command::SubCommandBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.option($crate::define_command!(@option($entry, $option_name, $option_kind, $locales, { $($body)* }))))*
    }};
    (@option<SubCommandGroup>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $($option_name:ident : $option_kind:ident { $($body:tt)* }),* $(,)?
    })) => {{
        ::twilight_util::builder::command::SubCommandGroupBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        .subcommands([$($crate::define_command!(@option($entry, $option_name, $option_kind, $locales, { $($body)* }))),*])
    }};
    (@option<User>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
    })) => {{
        ::twilight_util::builder::command::UserBuilder::new(
            $name,
            ::ina_localization::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
    }};
}
