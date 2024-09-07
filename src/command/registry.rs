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
#[non_exhaustive]
#[derive(Default)]
pub struct CommandRegistry {
    /// The inner command list.
    inner: HashMap<&'static str, CommandEntry>,
}

impl CommandRegistry {
    /// Creates a new [`CommandRegistry`].
    #[must_use]
    pub fn new() -> Self {
        Self { inner: HashMap::new() }
    }

    /// Returns whether this [`CommandRegistry`] contains a command with the given name.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.inner.contains_key(name)
    }

    /// Returns the command assigned to the given name from within this [`CommandRegistry`].
    #[must_use]
    pub fn command(&self, name: &str) -> Option<&CommandEntry> {
        self.inner.get(name)
    }

    /// Returns an iterator over references to the entries within this [`CommandRegistry`].
    pub fn iter(&self) -> impl Iterator<Item = &CommandEntry> {
        self.inner.values()
    }

    /// Returns an iterator over mutable references to the entries within this [`CommandRegistry`].
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

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_values()
    }
}

/// An entry within the command registry.
#[non_exhaustive]
pub struct CommandEntry {
    /// The command's literal name.
    pub name: &'static str,
    /// The command's creation function.
    pub factory: Box<dyn CommandFactory>,
    /// The command's callback functions.
    pub callbacks: CommandEntryCallbacks,
}

/// The callback functions of a [`CommandEntry`].
#[non_exhaustive]
#[derive(Default)]
pub struct CommandEntryCallbacks {
    /// The command callback.
    pub command: Option<Box<dyn CommandCallable>>,
    /// The component callback.
    pub component: Option<Box<dyn ComponentCallable>>,
    /// The modal callback.
    pub modal: Option<Box<dyn ModalCallable>>,
    /// The auto-completion callback.
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
    registry.register(super::definition::role::entry())?;

    drop(registry);

    info!(async "initialized command registry").await.map_err(Into::into)
}

/// Creates a command.
///
/// # Examples
///
/// ```
/// define_entry!("scream", CommandType::ChatInput, struct {
///     dev_only: false,
///     allow_dms: true,
///     is_nsfw: false,
///     permissions: Permissions::USE_SLASH_COMMANDS,
/// }, struct {
///     command: on_command,
/// }, struct {
///     ephemeral: Boolean {
///         required: true,
///     }
/// });
///
/// async fn on_command<'ap: 'ev, 'ev>(mut context: Context<'ap, 'ev, &'ev CommandData>) -> EventResult {
///     let resolver = CommandOptionResolver::new(context.state);
///     
///     context.text("AAAAAAAAAAAAAA", resolver.get_bool("ephemeral")?).await?;
///
///     crate::client::event::pass()
/// }
/// ```
#[macro_export]
macro_rules! define_entry {
    (
        $name:literal, $type:expr, struct {
            $(dev_only: $dev_only:literal,)?
            $(allow_dms: $allow_dms:literal,)?
            $(is_nsfw: $is_nsfw:literal,)?
            $(permissions: $permissions:expr,)?
        },struct {
            $(command: $command_callback:expr,)?
            $(component: $component_callback:expr,)?
            $(modal: $modal_callback:expr,)?
            $(autocomplete: $autocomplete_callback:expr,)?
        },struct { $($option_name:ident : $option_kind:ident { $($body:tt)* }),* $(,)? }
    ) => {
        /// The command implementation.
        pub struct Impl;

        #[::async_trait::async_trait]
        impl $crate::command::CommandFactory for Impl {
            async fn build(
                &self,
                entry: &$crate::command::registry::CommandEntry,
                guild_id: ::std::option::Option<::twilight_model::id::Id<::twilight_model::id::marker::GuildMarker>>,
            ) -> ::anyhow::Result<::std::option::Option<::twilight_model::application::command::Command>>
            {
                $(if $dev_only && guild_id.is_none() {
                    return ::std::result::Result::Ok(::std::option::Option::None);
                })?

                let localizer_name_key = ::std::format!("{}-name", entry.name);
                let localizer_description_key = ::std::format!("{}-description", entry.name);

                let localized_name = ::ina_localizing::localize!(async $crate::utility::category::COMMAND, &(*localizer_description_key)).await?;
                let mut builder = ::twilight_util::builder::command::CommandBuilder::new(entry.name, localized_name, $type);

                $(builder = builder.dm_permission($allow_dms);)?
                $(builder = builder.nsfw($is_nsfw);)?
                $(builder = builder.default_member_permissions($permissions);)?

                if let ::std::option::Option::Some(guild_id) = guild_id {
                    builder = builder.guild_id(guild_id);
                }

                let locales = ::ina_localizing::thread::list().await?;
                let mut localized_names = ::std::vec::Vec::with_capacity(locales.len());
                let mut localized_descriptions = ::std::vec::Vec::with_capacity(locales.len());

                for locale in &locales {
                    let name = <_ as ::std::string::ToString>::to_string(&::ina_localizing::localize!(async(in *locale) $crate::utility::category::COMMAND, &(*localizer_name_key)).await?);
                    let description = <_ as ::std::string::ToString>::to_string(&::ina_localizing::localize!(async(in *locale) $crate::utility::category::COMMAND, &(*localizer_description_key)).await?);

                    localized_names.push((<_ as ::std::string::ToString>::to_string(locale), name));
                    localized_descriptions.push((<_ as ::std::string::ToString>::to_string(locale), description));
                }

                builder = builder.name_localizations(localized_names);
                builder = builder.description_localizations(localized_descriptions);

                $(builder = builder.option($crate::define_entry!(@option(entry, $option_name, $option_kind, &locales, { $($body)* })));)*

                ::std::result::Result::Ok(::std::option::Option::Some(builder.validate()?.build()))
            }
        }

        $(
            #[::async_trait::async_trait]
            impl $crate::command::CommandCallable for Impl {
                async fn on_command<'ap: 'ev, 'ev>(
                    &self,
                    entry: &$crate::command::registry::CommandEntry,
                    context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::application_command::CommandData>,
                ) -> $crate::client::event::EventResult
                {
                    $command_callback(entry, context).await
                }
            }
        )?

        $(
            #[::async_trait::async_trait]
            impl $crate::command::ComponentCallable for Impl {
                async fn on_component<'ap: 'ev, 'ev>(
                    &self,
                    entry: &$crate::command::registry::CommandEntry,
                    context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::message_component::MessageComponentInteractionData>,
                    custom_id: $crate::utility::types::id::CustomId,
                ) -> $crate::client::event::EventResult
                {
                    $component_callback(entry, context, custom_id).await
                }
            }
        )?

        $(
            #[::async_trait::async_trait]
            impl $crate::command::ModalCallable for Impl {
                async fn on_modal<'ap: 'ev, 'ev>(
                    &self,
                    entry: &$crate::command::registry::CommandEntry,
                    context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::modal::ModalInteractionData>,
                    custom_id: $crate::utility::types::id::CustomId,
                ) -> $crate::client::event::EventResult
                {
                    $modal_callback(entry, context, custom_id).await
                }
            }
        )?

        $(
            #[::async_trait::async_trait]
            impl $crate::command::AutocompleteCallable for Impl {
                async fn on_autocomplete<'ap: 'ev, 'ev>(
                    &self,
                    entry: &$crate::command::registry::CommandEntry,
                    context: $crate::command::context::Context<'ap, 'ev, &'ev ::twilight_model::application::interaction::application_command::CommandData>,
                    option: &'ev ::std::primitive::str,
                    current: &'ev ::std::primitive::str,
                    kind: ::twilight_model::application::command::CommandOptionType,
                ) -> ::anyhow::Result<::std::boxed::Box<[::twilight_model::application::command::CommandOptionChoice]>>
                {
                    $autocomplete_callback(entry, context, option, current, kind).await
                }
            }
        )?

        /// Returns this command's registry entry.
        #[expect(clippy::allow_attributes, reason = "this is not always catching a lint")]
        #[must_use = r"command entries should be registered"]
        pub fn entry() -> $crate::command::registry::CommandEntry {
            #[allow(unused_mut)]
            let mut entry = $crate::command::registry::CommandEntry {
                name: $name,
                factory: ::std::boxed::Box::new(Impl),
                callbacks: <$crate::command::registry::CommandEntryCallbacks as ::std::default::Default>::default(),
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

        let mut builder = $crate::define_entry!(@option<$kind>($entry, name, &localizer_name_key, $locales, { $($body)* }));

        let mut localized_names = ::std::vec::Vec::with_capacity($locales.len());
        let mut localized_descriptions = ::std::vec::Vec::with_capacity($locales.len());

        for locale in $locales {
            let name = <_ as ::std::string::ToString>::to_string(&::ina_localizing::localize!(async(in *locale) $crate::utility::category::COMMAND_OPTION, &(*localizer_name_key)).await?);
            let description = <_ as ::std::string::ToString>::to_string(&::ina_localizing::localize!(async(in *locale) $crate::utility::category::COMMAND_OPTION, &(*localizer_description_key)).await?);

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
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
    }};
    (@option<Boolean>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
    })) => {{
        ::twilight_util::builder::command::BooleanBuilder::new(
            $name,
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
    }};
    (@option<Channel>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
        $(channel_types: $channel_types:expr,)?
    })) => {{
        ::twilight_util::builder::command::ChannelBuilder::new(
            $name,
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
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
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
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
                    let name = <_ as ::std::string::ToString>::to_string(&::ina_localizing::localize!(async(in *locale) $crate::utility::category::COMMAND_CHOICE, &(*localizer_key)).await?);

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
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
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
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
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
                    let name = <_ as ::std::string::ToString>::to_string(&::ina_localizing::localize!(async(in *locale) "choice", &(*localizer_key)).await?);

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
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
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
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
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
                    let name = <_ as ::std::string::ToString>::to_string(&::ina_localizing::localize!(async(in *locale) "choice", &(*localizer_key)).await?);

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
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.option($crate::define_entry!(@option($entry, $option_name, $option_kind, $locales, { $($body)* }))))*
    }};
    (@option<SubCommandGroup>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $($option_name:ident : $option_kind:ident { $($body:tt)* }),* $(,)?
    })) => {{
        ::twilight_util::builder::command::SubCommandGroupBuilder::new(
            $name,
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        .subcommands([$($crate::define_entry!(@option($entry, $option_name, $option_kind, $locales, { $($body)* }))),*])
    }};
    (@option<User>($entry:expr, $name:expr, $name_key:expr, $locales:expr, {
        $(required: $required:expr,)?
    })) => {{
        ::twilight_util::builder::command::UserBuilder::new(
            $name,
            ::ina_localizing::localize!(async $crate::utility::category::COMMAND_OPTION, $name_key.as_str()).await?
        )
        $(.required($required))?
    }};
}
