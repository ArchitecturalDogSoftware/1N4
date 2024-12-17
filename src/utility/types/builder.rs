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

use twilight_model::channel::ChannelType;
use twilight_model::channel::message::component::{
    ActionRow, Button, ButtonStyle, SelectDefaultValue, SelectMenu, SelectMenuOption, SelectMenuType, TextInput,
    TextInputStyle,
};
use twilight_model::channel::message::{Component, EmojiReactionType};
use twilight_model::id::Id;
use twilight_model::id::marker::SkuMarker;
use twilight_validate::component::{
    ACTION_ROW_COMPONENT_COUNT, COMPONENT_BUTTON_LABEL_LENGTH, COMPONENT_CUSTOM_ID_LENGTH, SELECT_MAXIMUM_VALUES_LIMIT,
    SELECT_MAXIMUM_VALUES_REQUIREMENT, SELECT_MINIMUM_VALUES_LIMIT, SELECT_OPTION_COUNT,
    SELECT_OPTION_DESCRIPTION_LENGTH, SELECT_OPTION_LABEL_LENGTH, SELECT_OPTION_VALUE_LENGTH,
    SELECT_PLACEHOLDER_LENGTH, TEXT_INPUT_LABEL_MAX, TEXT_INPUT_LABEL_MIN, TEXT_INPUT_LENGTH_MAX,
    TEXT_INPUT_LENGTH_MIN, TEXT_INPUT_PLACEHOLDER_MAX,
};

/// An error that may be returned when interacting with builders.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Returned when a method is used on an invalid builder type.
    #[error("the '{0}' method is not applicable for the specified type")]
    InvalidType(&'static str),
    /// Returned when a method is given an invalid value.
    #[error("an invalid value was provided")]
    InvalidValue,
    /// Returned when a limit is exceeded.
    #[error("{0} limit exceeded: {1}/{2}")]
    LimitExceeded(&'static str, usize, usize),
}

/// Builds an [`ActionRow`].
#[must_use = "builders must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionRowBuilder(ActionRow);

impl ActionRowBuilder {
    /// Creates a new [`ActionRowBuilder`].
    pub const fn new() -> Self {
        Self(ActionRow { components: Vec::new() })
    }

    /// Adds a component to the action row.
    ///
    /// # Errors
    ///
    /// This function will return an error if the maximum number of permitted components is exceeded.
    pub fn component(mut self, component: impl Into<Component>) -> Result<Self, Error> {
        if self.0.components.len() >= ACTION_ROW_COMPONENT_COUNT {
            return Err(Error::LimitExceeded("component", self.0.components.len(), ACTION_ROW_COMPONENT_COUNT));
        }

        self.0.components.push(component.into());

        Ok(self)
    }

    /// Builds the completed action row.
    #[must_use]
    pub fn build(self) -> ActionRow {
        self.0
    }
}

impl From<ActionRowBuilder> for ActionRow {
    fn from(value: ActionRowBuilder) -> Self {
        value.build()
    }
}

impl From<ActionRowBuilder> for Component {
    fn from(value: ActionRowBuilder) -> Self {
        Self::ActionRow(value.build())
    }
}

impl<I: Into<Component>> FromIterator<I> for ActionRowBuilder {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Self(ActionRow { components: iter.into_iter().take(5).map(Into::into).collect() })
    }
}

impl Default for ActionRowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds a [`Button`].
#[must_use = "builders must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ButtonBuilder(Button);

impl ButtonBuilder {
    /// Creates a new [`ButtonBuilder`].
    pub const fn new(style: ButtonStyle) -> Self {
        Self(Button { custom_id: None, disabled: false, emoji: None, label: None, style, url: None, sku_id: None })
    }

    /// Sets the button's custom identifier.
    ///
    /// # Errors
    ///
    /// This function will return an error if the custom identifier is too long, or if this is used on a link or premium
    /// button.
    pub fn custom_id(mut self, custom_id: impl Into<String>) -> Result<Self, Error> {
        if matches!(self.0.style, ButtonStyle::Link | ButtonStyle::Premium) {
            return Err(Error::InvalidType("custom_id"));
        }

        let custom_id: String = custom_id.into();

        if custom_id.len() > COMPONENT_CUSTOM_ID_LENGTH {
            return Err(Error::LimitExceeded("identifier length", custom_id.len(), COMPONENT_CUSTOM_ID_LENGTH));
        }

        self.0.custom_id = Some(custom_id);

        Ok(self)
    }

    /// Sets whether the button is disabled.
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.0.disabled = disabled;

        self
    }

    /// Sets the button's emoji.
    ///
    /// # Errors
    ///
    /// This function will return an error if this is used on a premium button.
    pub fn emoji(mut self, emoji: impl Into<EmojiReactionType>) -> Result<Self, Error> {
        if matches!(self.0.style, ButtonStyle::Premium) {
            return Err(Error::InvalidType("emoji"));
        }

        self.0.emoji = Some(emoji.into());

        Ok(self)
    }

    /// Sets the button's label text.
    ///
    /// # Errors
    ///
    /// This function will return an error if the label exceeds the limit.
    pub fn label(mut self, label: impl Into<String>) -> Result<Self, Error> {
        if matches!(self.0.style, ButtonStyle::Premium) {
            return Err(Error::InvalidType("label"));
        }

        let label: String = label.into();

        if label.len() > COMPONENT_BUTTON_LABEL_LENGTH {
            return Err(Error::LimitExceeded("label length", label.len(), COMPONENT_BUTTON_LABEL_LENGTH));
        }

        self.0.label = Some(label);

        Ok(self)
    }

    /// Sets the button's URL.
    ///
    /// # Errors
    ///
    /// This function will return an error if the button is not a link button.
    pub fn url(mut self, url: impl Into<String>) -> Result<Self, Error> {
        if !matches!(self.0.style, ButtonStyle::Link) {
            return Err(Error::InvalidType("url"));
        }

        self.0.url = Some(url.into());

        Ok(self)
    }

    /// Sets the button's SKU identifier.
    ///
    /// # Errors
    ///
    /// This function will return an error if the button is not a premium button.
    pub fn sku_id(mut self, id: Id<SkuMarker>) -> Result<Self, Error> {
        if !matches!(self.0.style, ButtonStyle::Premium) {
            return Err(Error::InvalidType("sku_id"));
        }

        self.0.sku_id = Some(id);

        Ok(self)
    }

    /// Builds the completed button.
    #[must_use]
    pub fn build(self) -> Button {
        self.0
    }
}

impl From<ButtonBuilder> for Button {
    fn from(value: ButtonBuilder) -> Self {
        value.build()
    }
}

impl From<ButtonBuilder> for Component {
    fn from(value: ButtonBuilder) -> Self {
        Self::Button(value.build())
    }
}

/// Builds a [`SelectMenu`].
#[must_use = "builders must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectMenuBuilder(SelectMenu);

impl SelectMenuBuilder {
    /// Creates a new [`SelectMenuBuilder`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the custom identifier is too long.
    pub fn new(custom_id: impl Into<String>, kind: SelectMenuType) -> Result<Self, Error> {
        let custom_id: String = custom_id.into();

        if custom_id.len() > COMPONENT_CUSTOM_ID_LENGTH {
            return Err(Error::LimitExceeded("identifier length", custom_id.len(), COMPONENT_CUSTOM_ID_LENGTH));
        }

        Ok(Self(SelectMenu {
            channel_types: None,
            custom_id,
            default_values: None,
            disabled: false,
            kind,
            max_values: None,
            min_values: None,
            options: None,
            placeholder: None,
        }))
    }

    /// Adds a channel type to the list.
    ///
    /// # Errors
    ///
    /// This function will return an error if this is used on a non-channel selection menu.
    pub fn channel_type(mut self, channel_type: ChannelType) -> Result<Self, Error> {
        if !matches!(self.0.kind, SelectMenuType::Channel) {
            return Err(Error::InvalidType("channel_type"));
        }

        let list = self.0.channel_types.get_or_insert(Vec::with_capacity(1));

        list.push(channel_type);
        list.dedup();

        Ok(self)
    }

    /// Adds a default value to the list.
    ///
    /// # Errors
    ///
    /// This function will return an error if this is used on a text selection menu, or if the given value is invalid.
    pub fn default_value(mut self, value: impl Into<SelectDefaultValue>) -> Result<Self, Error> {
        if matches!(self.0.kind, SelectMenuType::Text) {
            return Err(Error::InvalidType("default_value"));
        }

        let value: SelectDefaultValue = value.into();
        let value_kind_matches = match value {
            SelectDefaultValue::User(_) => self.0.kind == SelectMenuType::User,
            SelectDefaultValue::Role(_) => self.0.kind == SelectMenuType::Role,
            SelectDefaultValue::Channel(_) => self.0.kind == SelectMenuType::Channel,
        };

        if self.0.kind != SelectMenuType::Mentionable && !value_kind_matches {
            return Err(Error::InvalidValue);
        }

        let list = self.0.default_values.get_or_insert(Vec::with_capacity(1));
        let max_len = self.0.max_values.unwrap_or(1).into();

        if list.len() > max_len {
            return Err(Error::LimitExceeded("default value", list.len(), max_len));
        }

        list.push(value);
        list.dedup();

        Ok(self)
    }

    /// Sets whether the selection menu is disabled.
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.0.disabled = disabled;

        self
    }

    /// Sets the selection menu's maximum number of select-able items.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value is outside of the valid range.
    pub fn max_values(mut self, max: u8) -> Result<Self, Error> {
        if (max as usize) < SELECT_MAXIMUM_VALUES_REQUIREMENT {
            return Err(Error::LimitExceeded("value", max as usize, SELECT_MAXIMUM_VALUES_REQUIREMENT));
        }
        if (max as usize) > SELECT_MAXIMUM_VALUES_LIMIT {
            return Err(Error::LimitExceeded("value", max as usize, SELECT_MAXIMUM_VALUES_LIMIT));
        }

        self.0.max_values = Some(max);

        Ok(self)
    }

    /// Sets the selection menu's minimum number of select-able items.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value is outside of the valid range.
    pub fn min_values(mut self, min: u8) -> Result<Self, Error> {
        if (min as usize) < SELECT_MAXIMUM_VALUES_REQUIREMENT {
            return Err(Error::LimitExceeded("value", min as usize, SELECT_MAXIMUM_VALUES_REQUIREMENT));
        }
        if (min as usize) > SELECT_MINIMUM_VALUES_LIMIT {
            return Err(Error::LimitExceeded("value", min as usize, SELECT_MINIMUM_VALUES_LIMIT));
        }

        self.0.min_values = Some(min);

        Ok(self)
    }

    /// Adds an option to the list.
    ///
    /// # Errors
    ///
    /// This function will return an error if this is used on a non-text selection menu, or if the total number of
    /// options exceeds the limit.
    pub fn option(mut self, option: impl Into<SelectMenuOption>) -> Result<Self, Error> {
        if !matches!(self.0.kind, SelectMenuType::Text) {
            return Err(Error::InvalidType("option"));
        }

        let option: SelectMenuOption = option.into();
        let list = self.0.options.get_or_insert(Vec::with_capacity(1));

        if list.len() > SELECT_OPTION_COUNT {
            return Err(Error::LimitExceeded("option", list.len(), SELECT_OPTION_COUNT));
        }

        list.push(option);
        list.dedup();

        Ok(self)
    }

    /// Sets the selection menu's placeholder text.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value's length exceeds the limit.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Result<Self, Error> {
        let placeholder: String = placeholder.into();

        if placeholder.len() > SELECT_PLACEHOLDER_LENGTH {
            return Err(Error::LimitExceeded("placeholder length", placeholder.len(), SELECT_PLACEHOLDER_LENGTH));
        }

        self.0.placeholder = Some(placeholder);

        Ok(self)
    }

    /// Builds the completed selection menu.
    #[must_use]
    pub fn build(self) -> SelectMenu {
        self.0
    }
}

impl From<SelectMenuBuilder> for SelectMenu {
    fn from(value: SelectMenuBuilder) -> Self {
        value.build()
    }
}

impl From<SelectMenuBuilder> for Component {
    fn from(value: SelectMenuBuilder) -> Self {
        Self::SelectMenu(value.build())
    }
}

/// Builds a [`SelectMenuOption`].
#[must_use = "builders must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectMenuOptionBuilder(SelectMenuOption);

impl SelectMenuOptionBuilder {
    /// Creates a new [`SelectMenuOptionBuilder`].
    ///
    /// # Errors
    ///
    /// This function will return an error if a value exceeds the character limit.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Result<Self, Error> {
        let label: String = label.into();

        if label.len() > SELECT_OPTION_LABEL_LENGTH {
            return Err(Error::LimitExceeded("label length", label.len(), SELECT_OPTION_LABEL_LENGTH));
        }

        let value: String = value.into();

        if value.len() > SELECT_OPTION_VALUE_LENGTH {
            return Err(Error::LimitExceeded("value length", value.len(), SELECT_OPTION_VALUE_LENGTH));
        }

        Ok(Self(SelectMenuOption { default: false, description: None, emoji: None, label, value }))
    }

    /// Sets whether the option is selected by default.
    pub const fn default(mut self, default: bool) -> Self {
        self.0.default = default;

        self
    }

    /// Sets the option's description.
    ///
    /// # Errors
    ///
    /// This function will return an error if the description exceeds the character limit.
    pub fn description(mut self, description: impl Into<String>) -> Result<Self, Error> {
        let description: String = description.into();

        if description.len() > SELECT_OPTION_DESCRIPTION_LENGTH {
            return Err(Error::LimitExceeded(
                "description length",
                description.len(),
                SELECT_OPTION_DESCRIPTION_LENGTH,
            ));
        }

        self.0.description = Some(description);

        Ok(self)
    }

    /// Sets the option's emoji.
    pub fn emoji(mut self, emoji: impl Into<EmojiReactionType>) -> Self {
        self.0.emoji = Some(emoji.into());

        self
    }

    /// Builds the completed selection menu option.
    #[must_use]
    pub fn build(self) -> SelectMenuOption {
        self.0
    }
}

impl From<SelectMenuOptionBuilder> for SelectMenuOption {
    fn from(value: SelectMenuOptionBuilder) -> Self {
        value.build()
    }
}

/// Builds a [`TextInput`].
#[must_use = "builders must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInputBuilder(TextInput);

impl TextInputBuilder {
    /// Creates a new [`TextInputBuilder`].
    ///
    /// # Errors
    ///
    /// This function will return an error if a value exceeds the character limit.
    pub fn new(custom_id: impl Into<String>, label: impl Into<String>, style: TextInputStyle) -> Result<Self, Error> {
        let custom_id: String = custom_id.into();

        if custom_id.len() > COMPONENT_CUSTOM_ID_LENGTH {
            return Err(Error::LimitExceeded("identifier length", custom_id.len(), COMPONENT_CUSTOM_ID_LENGTH));
        }

        let label: String = label.into();

        if label.len() < TEXT_INPUT_LABEL_MIN {
            return Err(Error::LimitExceeded("label length", label.len(), TEXT_INPUT_LABEL_MIN));
        } else if label.len() > TEXT_INPUT_LABEL_MAX {
            return Err(Error::LimitExceeded("label length", label.len(), TEXT_INPUT_LABEL_MAX));
        }

        Ok(Self(TextInput {
            custom_id,
            label,
            max_length: None,
            min_length: None,
            placeholder: None,
            required: None,
            style,
            value: None,
        }))
    }

    /// Sets the text input's maximum input length.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value is outside of the valid range.
    pub fn max_length(mut self, max: u16) -> Result<Self, Error> {
        if (max as usize) < TEXT_INPUT_LENGTH_MIN {
            return Err(Error::LimitExceeded("value", max as usize, TEXT_INPUT_LENGTH_MIN));
        } else if (max as usize) > TEXT_INPUT_LENGTH_MAX {
            return Err(Error::LimitExceeded("value", max as usize, TEXT_INPUT_LENGTH_MAX));
        }

        self.0.max_length = Some(max);

        Ok(self)
    }

    /// Sets the text input's minimum input length.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value is outside of the valid range.
    pub fn min_length(mut self, min: u16) -> Result<Self, Error> {
        if (min as usize) < TEXT_INPUT_LENGTH_MIN {
            return Err(Error::LimitExceeded("value", min as usize, TEXT_INPUT_LENGTH_MIN));
        } else if (min as usize) > TEXT_INPUT_LENGTH_MAX {
            return Err(Error::LimitExceeded("value", min as usize, TEXT_INPUT_LENGTH_MAX));
        }

        self.0.min_length = Some(min);

        Ok(self)
    }

    /// Sets the text input's placeholder text.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value's length exceeds the limit.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Result<Self, Error> {
        let placeholder: String = placeholder.into();

        if placeholder.len() > TEXT_INPUT_PLACEHOLDER_MAX {
            return Err(Error::LimitExceeded("placeholder length", placeholder.len(), TEXT_INPUT_PLACEHOLDER_MAX));
        }

        self.0.placeholder = Some(placeholder);

        Ok(self)
    }

    /// Sets whether the button is required.
    pub const fn required(mut self, required: bool) -> Self {
        self.0.required = Some(required);

        self
    }

    /// Sets the text input's pre-filled value text.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value's length exceeds the limit.
    pub fn value(mut self, value: impl Into<String>) -> Result<Self, Error> {
        let value: String = value.into();
        let max_len = self.0.max_length.map_or(TEXT_INPUT_LENGTH_MAX, |v| v as usize);

        if value.len() > max_len {
            return Err(Error::LimitExceeded("value length", value.len(), max_len));
        }

        self.0.value = Some(value);

        Ok(self)
    }

    /// Builds the completed text input.
    #[must_use]
    pub fn build(self) -> TextInput {
        self.0
    }
}

impl From<TextInputBuilder> for TextInput {
    fn from(value: TextInputBuilder) -> Self {
        value.build()
    }
}

impl From<TextInputBuilder> for Component {
    fn from(value: TextInputBuilder) -> Self {
        Self::TextInput(value.build())
    }
}
