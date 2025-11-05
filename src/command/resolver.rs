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

    /// Returns a reference to the stored attachment identifier associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not an
    /// attachment identifier.
    pub fn attachment_id(&'ev self, name: impl AsRef<str>) -> Result<&'ev Id<AttachmentMarker>, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::Attachment(value) => Ok(value),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::Attachment, other.kind())),
        }
    }

    /// Returns a reference to the stored boolean associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a boolean.
    pub fn boolean(&'ev self, name: impl AsRef<str>) -> Result<&'ev bool, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::Boolean(value) => Ok(value),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::Boolean, other.kind())),
        }
    }

    /// Returns a reference to the stored channel identifier associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a channel
    /// identifier.
    pub fn channel_id(&'ev self, name: impl AsRef<str>) -> Result<&'ev Id<ChannelMarker>, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::Channel(value) => Ok(value),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::Channel, other.kind())),
        }
    }

    /// Returns a reference to the stored float associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a float.
    pub fn float(&'ev self, name: impl AsRef<str>) -> Result<&'ev f64, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::Number(value) => Ok(value),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::Number, other.kind())),
        }
    }

    /// Returns a reference to the stored integer associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not an integer.
    pub fn integer(&'ev self, name: impl AsRef<str>) -> Result<&'ev i64, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::Integer(value) => Ok(value),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::Integer, other.kind())),
        }
    }

    /// Returns a reference to the stored mentionable identifier associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a
    /// mentionable identifier.
    pub fn mentionable_id(&'ev self, name: impl AsRef<str>) -> Result<&'ev Id<GenericMarker>, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::Mentionable(value) => Ok(value),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::Mentionable, other.kind())),
        }
    }

    /// Returns a reference to the stored role identifier associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a role
    /// identifier.
    pub fn role_id(&'ev self, name: impl AsRef<str>) -> Result<&'ev Id<RoleMarker>, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::Role(value) => Ok(value),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::Role, other.kind())),
        }
    }

    /// Returns a reference to the stored string associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a string.
    pub fn string(&'ev self, name: impl AsRef<str>) -> Result<&'ev str, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::String(value) => Ok(value),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::String, other.kind())),
        }
    }

    /// Returns a reference to the stored user identifier associated with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist, or if the associated option is not a user
    /// identifier.
    pub fn user_id(&'ev self, name: impl AsRef<str>) -> Result<&'ev Id<UserMarker>, Error> {
        let name = name.as_ref();

        match self.any(name)? {
            CommandOptionValue::User(value) => Ok(value),
            other => Err(Error::InvalidOption(name.into(), CommandOptionType::User, other.kind())),
        }
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
