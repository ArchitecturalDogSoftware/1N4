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

use twilight_model::application::command::CommandOptionType;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::modal::{ModalInteractionData, ModalInteractionDataActionRow};
use twilight_model::id::marker::{AttachmentMarker, ChannelMarker, GenericMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;

/// An error that may be returned when interacting with resolvers.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Returned if an option is of an invalid type.
    #[error("the option '{0}' is invalid (expected '{1:?}', found '{2:?}')")]
    InvalidOption(Box<str>, CommandOptionType, CommandOptionType),
    /// Returned if a field is missing from the resolver.
    #[error("the field '{0}' is missing")]
    MissingField(Box<str>),
    /// Returned if an option is missing from the resolver.
    #[error("the option '{0}' is missing")]
    MissingOption(Box<str>),
}

/// Resolves and caches a command's defined options.
#[must_use = "this type should be used to resolve command options"]
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct CommandOptionResolver<'ev> {
    /// The command's data.
    data: &'ev CommandData,
    /// The cached options.
    options: HashMap<&'ev str, &'ev CommandOptionValue>,
}

impl<'ev> CommandOptionResolver<'ev> {
    /// Creates a new [`CommandOptionResolver`].
    pub fn new(data: &'ev CommandData) -> Self {
        Self::with_options(data, &data.options)
    }

    /// Creates a new [`CommandOptionResolver`] using the given option list.
    fn with_options(data: &'ev CommandData, options: impl IntoIterator<Item = &'ev CommandDataOption>) -> Self {
        Self { data, options: options.into_iter().map(|o| (&(*o.name), &o.value)).collect() }
    }

    /// Returns a reference to the stored [`CommandOptionValue`] associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist.
    pub fn any(&'ev self, name: impl AsRef<str>) -> Result<&'ev CommandOptionValue, Error> {
        let name = name.as_ref();

        self.options.get(name).copied().ok_or_else(|| Error::MissingOption(name.into()))
    }

    /// Returns a new [`CommandOptionResolver`] for the options assigned to the subcommand associated with the given
    /// name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the subcommand does not exist, or if the associated option is not a
    /// subcommand.
    pub fn subcommand(&'ev self, name: impl AsRef<str>) -> Result<Self, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::SubCommand(ref options) => Ok(Self::with_options(self.data, options)),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::SubCommand, other.kind())),
        }
    }

    /// Returns a new [`CommandOptionResolver`] for the subcommands assigned to the subcommand group associated with the
    /// given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the subcommand group does not exist, or if the associated option is not a
    /// subcommand group.
    pub fn subcommand_group(&'ev self, name: impl AsRef<str>) -> Result<Self, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::SubCommandGroup(ref options) => Ok(Self::with_options(self.data, options)),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::SubCommandGroup, other.kind())),
        }
    }
}

macro_rules! command_option_resolver_getters {
    ($($(#[$attribute:meta])* $name:ident as $type:ident -> $return:ty;)*) => {
        impl<'ev> CommandOptionResolver<'ev> {$(
            $(#[$attribute])*
            pub fn $name(&'ev self, name: impl AsRef<str>) -> Result<$return, Error> {
                let name = name.as_ref();

                match self.any(name)? {
                    CommandOptionValue::$type(ref value) => Ok(value),
                    other => Err(Error::InvalidOption(name.into(), CommandOptionType::$type, other.kind())),
                }
            }
        )*}
    };
}

command_option_resolver_getters! {
    /// Returns a reference to the stored attachment identifier associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not an
    /// attachment identifier.
    attachment_id as Attachment -> &'ev Id<AttachmentMarker>;

    /// Returns a reference to the stored boolean associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a boolean.
    boolean as Boolean -> &'ev bool;

    /// Returns a reference to the stored channel identifier associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a channel
    /// identifier.
    channel_id as Channel -> &'ev Id<ChannelMarker>;

    /// Returns a reference to the stored float associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a float.
    float as Number -> &'ev f64;

    /// Returns a reference to the stored integer associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not an integer.
    integer as Integer -> &'ev i64;

    /// Returns a reference to the stored mentionable identifier associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a
    /// mentionable identifier.
    mentionable_id as Mentionable -> &'ev Id<GenericMarker>;

    /// Returns a reference to the stored role identifier associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a role
    /// identifier.
    role_id as Role -> &'ev Id<RoleMarker>;

    /// Returns a reference to the stored string associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a string.
    string as String -> &'ev str;

    /// Returns a reference to the stored user identifier associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a user
    /// identifier.
    user_id as User -> &'ev Id<UserMarker>;
}

/// Resolves and caches a modal's defined fields.
#[must_use = "this type should be used to resolve modal fields"]
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalFieldResolver<'ev> {
    /// The modal's data.
    data: &'ev ModalInteractionData,
    /// The cached fields.
    fields: HashMap<&'ev str, Option<&'ev str>>,
}

impl<'ev> ModalFieldResolver<'ev> {
    /// Creates a new [`ModalFieldResolver`].
    pub fn new(data: &'ev ModalInteractionData) -> Self {
        Self::with_fields(data, &data.components)
    }

    /// Creates a new [`ModalFieldResolver`] using the given field list.
    fn with_fields(
        data: &'ev ModalInteractionData,
        fields: impl IntoIterator<Item = &'ev ModalInteractionDataActionRow>,
    ) -> Self {
        let fields = fields.into_iter().filter_map(|r| {
            // Do we want to keep this saved as `Option<_>`?
            // It *does* better differentiate between not present and left empty.
            r.components.first().map(|c| (&(*c.custom_id), c.value.as_deref()))
        });

        Self { data, fields: fields.collect() }
    }

    /// Returns a reference to the stored value associated with the given field name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the field does not exist.
    pub fn get(&self, name: impl AsRef<str>) -> Result<Option<&str>, Error> {
        let name = name.as_ref();

        self.fields.get(name).copied().ok_or_else(|| Error::MissingField(name.into()))
    }
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
