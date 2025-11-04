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

use twilight_model::channel::message::Component;
use twilight_model::channel::message::component::{TextInput, TextInputStyle};
use twilight_validate::component::{
    COMPONENT_CUSTOM_ID_LENGTH, TEXT_INPUT_LABEL_MAX, TEXT_INPUT_LABEL_MIN, TEXT_INPUT_LENGTH_MAX,
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
            id: None,
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

    /// Sets the text input's numeric identifier.
    pub const fn id(mut self, id: i32) -> Self {
        self.0.id = Some(id);
        self
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
