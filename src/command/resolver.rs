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

use std::collections::{BTreeMap, HashMap};

use twilight_model::application::command::CommandOptionType;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::modal::{
    ModalInteractionChannelSelect, ModalInteractionComponent, ModalInteractionData, ModalInteractionFileUpload,
    ModalInteractionMentionableSelect, ModalInteractionRoleSelect, ModalInteractionStringSelect,
    ModalInteractionTextInput, ModalInteractionUserSelect,
};
use twilight_model::channel::message::component::ComponentType;
use twilight_model::id::Id;
use twilight_model::id::marker::{AttachmentMarker, ChannelMarker, GenericMarker, RoleMarker, UserMarker};

use crate::utility::traits::extension::ModalInteractionComponentExt;

/// An error that may be returned when interacting with resolvers.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Returned if a component is of an invalid type.
    #[error("the modal component '{0}' is invalid (expected '{1:?}', found '{2:?}')")]
    InvalidComponent(i32, ComponentType, ComponentType),
    /// Returned if an option is of an invalid type.
    #[error("the option '{0}' is invalid (expected '{1:?}', found '{2:?}')")]
    InvalidOption(Box<str>, CommandOptionType, CommandOptionType),
    /// Returned if a component is missing from the resolver.
    #[error("the modal component '{0}' is missing")]
    MissingComponent(i32),
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
    pub data: &'ev CommandData,
    /// The cached options.
    options: HashMap<&'ev str, &'ev CommandOptionValue>,
}

impl<'ev> CommandOptionResolver<'ev> {
    /// Creates a new [`CommandOptionResolver`].
    pub fn new(data: &'ev CommandData) -> Self {
        Self::with_options(data, &data.options)
    }

    /// Creates a new [`CommandOptionResolver`] using the given option list.
    fn with_options<I>(data: &'ev CommandData, options: I) -> Self
    where
        I: IntoIterator<Item = &'ev CommandDataOption>,
    {
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
            CommandOptionValue::SubCommand(options) => Ok(Self::with_options(self.data, options)),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::SubCommand, other.kind())),
        }
    }

    /// Returns a new [`CommandOptionResolver`] for the sub-commands assigned to the subcommand group associated with
    /// the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the subcommand group does not exist, or if the associated option is not a
    /// subcommand group.
    pub fn subcommand_group(&'ev self, name: impl AsRef<str>) -> Result<Self, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::SubCommandGroup(options) => Ok(Self::with_options(self.data, options)),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::SubCommandGroup, other.kind())),
        }
    }
}

/// Defines getter functions for the [`CommandOptionResolver`] type.
///
/// # Examples
///
/// ```
/// command_option_resolver_getters! {
///    /// Returns a reference to the stored boolean associated with the given name.
///    ///
///    /// # Errors
///    ///
///    /// This function will return an error if the option does not exist, or if the associated option is not a boolean.
///    boolean as Boolean -> &'ev bool;
/// }
///
/// // generates a function with the following signature:
/// //
/// // fn boolean<'ev>(&'ev self, name: impl AsRef<str>) -> Result<&'ev bool, Error>;
/// ```
macro_rules! command_option_resolver_getters {
    ($($(#[$attribute:meta])* $name:ident as $type:ident -> $return:ty;)*) => {
        impl<'ev> CommandOptionResolver<'ev> {$(
            $(#[$attribute])*
            pub fn $name(&'ev self, name: impl AsRef<str>) -> Result<$return, Error> {
                let name = name.as_ref();

                match self.any(name)? {
                    CommandOptionValue::$type(value) => Ok(value),
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
#[derive(Clone, Debug, PartialEq)]
pub struct ModalComponentResolver<'ev> {
    /// The modal's data.
    pub data: &'ev ModalInteractionData,
    /// The modal's cached components.
    components: BTreeMap<i32, &'ev ModalInteractionComponent>,
}

impl<'ev> ModalComponentResolver<'ev> {
    /// Creates a new [`ModalComponentResolver`].
    pub fn new(data: &'ev ModalInteractionData) -> Self {
        Self::with_components(data, &data.components)
    }

    /// Creates a new [`ModalComponentResolver`] using the given field list.
    fn with_components<I>(data: &'ev ModalInteractionData, components: I) -> Self
    where
        I: IntoIterator<Item = &'ev ModalInteractionComponent>,
    {
        Self { data, components: components.into_iter().map(|component| (component.id(), component)).collect() }
    }

    /// Returns a reference to the stored [`ModalInteractionComponent`] associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component does not exist.
    pub fn any(&'ev self, id: i32) -> Result<&'ev ModalInteractionComponent, Error> {
        self.components.get(&id).copied().ok_or(Error::MissingComponent(id))
    }

    /// Returns a new [`ModalComponentResolver`] containing all components within the stored action row.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component does not exist or is not an action row.
    pub fn action_row(&'ev self, id: i32) -> Result<Self, Error> {
        match self.any(id)? {
            ModalInteractionComponent::ActionRow(value) => Ok(Self::with_components(self.data, &(*value.components))),
            other => Err(Error::InvalidComponent(id, ComponentType::ActionRow, other.kind())),
        }
    }

    /// Returns a new [`ModalComponentResolver`] containing all components within the stored label.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component does not exist or is not a label.
    pub fn label(&'ev self, id: i32) -> Result<Self, Error> {
        match self.any(id)? {
            ModalInteractionComponent::Label(value) => {
                Ok(Self::with_components(self.data, std::iter::once(&(*value.component))))
            }
            other => Err(Error::InvalidComponent(id, ComponentType::Label, other.kind())),
        }
    }

    /// Returns a reference to the stored string selector associated with the given numeric identifier.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component does not exist, or if the associated component is not a
    /// string selector.
    pub fn string_select(&'ev self, id: i32) -> Result<&'ev ModalInteractionStringSelect, Error> {
        match self.any(id)? {
            ModalInteractionComponent::StringSelect(value) => Ok(value),
            other => Err(Error::InvalidComponent(id, ComponentType::TextSelectMenu, other.kind())),
        }
    }

    /// Returns a reference to the stored text input associated with the given numeric identifier.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component does not exist, or if the associated component is not a text
    /// input.
    pub fn text_input(&'ev self, id: i32) -> Result<&'ev ModalInteractionTextInput, Error> {
        match self.any(id)? {
            ModalInteractionComponent::TextInput(value) => Ok(value),
            other => Err(Error::InvalidComponent(id, ComponentType::TextInput, other.kind())),
        }
    }

    /// Returns a reference to the stored user selector associated with the given numeric identifier.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component does not exist, or if the associated component is not a user
    /// selector.
    pub fn user_select(&'ev self, id: i32) -> Result<&'ev ModalInteractionUserSelect, Error> {
        match self.any(id)? {
            ModalInteractionComponent::UserSelect(value) => Ok(value),
            other => Err(Error::InvalidComponent(id, ComponentType::UserSelectMenu, other.kind())),
        }
    }

    /// Returns a reference to the stored role selector associated with the given numeric identifier.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component does not exist, or if the associated component is not a role
    /// selector.
    pub fn role_select(&'ev self, id: i32) -> Result<&'ev ModalInteractionRoleSelect, Error> {
        match self.any(id)? {
            ModalInteractionComponent::RoleSelect(value) => Ok(value),
            other => Err(Error::InvalidComponent(id, ComponentType::RoleSelectMenu, other.kind())),
        }
    }

    /// Returns a reference to the stored mentionable selector associated with the given numeric identifier.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component does not exist, or if the associated component is not a
    /// mentionable selector.
    pub fn mentionable_select(&'ev self, id: i32) -> Result<&'ev ModalInteractionMentionableSelect, Error> {
        match self.any(id)? {
            ModalInteractionComponent::MentionableSelect(value) => Ok(value),
            other => Err(Error::InvalidComponent(id, ComponentType::MentionableSelectMenu, other.kind())),
        }
    }

    /// Returns a reference to the stored channel selector associated with the given numeric identifier.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component does not exist, or if the associated component is not a
    /// channel selector.
    pub fn channel_select(&'ev self, id: i32) -> Result<&'ev ModalInteractionChannelSelect, Error> {
        match self.any(id)? {
            ModalInteractionComponent::ChannelSelect(value) => Ok(value),
            other => Err(Error::InvalidComponent(id, ComponentType::ChannelSelectMenu, other.kind())),
        }
    }

    /// Returns a reference to the stored file upload associated with the given numeric identifier.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component does not exist, or if the associated component is not a file
    /// upload.
    pub fn file_upload(&'ev self, id: i32) -> Result<&'ev ModalInteractionFileUpload, Error> {
        match self.any(id)? {
            ModalInteractionComponent::FileUpload(value) => Ok(value),
            other => Err(Error::InvalidComponent(id, ComponentType::FileUpload, other.kind())),
        }
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
