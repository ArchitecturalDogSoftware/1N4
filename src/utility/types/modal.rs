// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024—2025 Jaxydog
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

use serde::{Deserialize, Serialize};
use twilight_model::channel::message::Component;
use twilight_model::channel::message::component::{ActionRow, TextInput};
use twilight_validate::component::COMPONENT_CUSTOM_ID_LENGTH;

/// The maximum amount of permitted inputs within a single modal.
pub const MODAL_INPUT_COUNT: usize = 5;
/// The maximum length of a modal title.
pub const MODAL_TITLE_LENGTH: usize = 45;

/// An error that may be returned when interacting with modals.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An invalid custom identifier was provided.
    #[error("an invalid custom identifier was provided: '{0}'")]
    InvalidCustomId(Box<str>),
    /// An invalid title was provided.
    #[error("an invalid title was provided: '{0}'")]
    InvalidTitle(Box<str>),
    /// Returned when attempting to add more than the allowed number of inputs.
    #[error("a maximum of {MODAL_INPUT_COUNT} inputs is permitted")]
    MaximumInputs,
    /// Returned when no inputs are provided.
    #[error("a minimum of 1 input is required")]
    MissingInput,
}

/// A modal's basic data.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModalData {
    /// The modal's custom identifier.
    pub custom_id: Box<str>,
    /// The modal's title.
    pub title: Box<str>,
    /// The modal's components.
    pub components: Box<[Component]>,
}

impl ModalData {
    /// Creates a new [`ModalData`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the data is invalid.
    pub fn new(
        custom_id: impl Into<String>,
        title: impl Into<String>,
        components: impl IntoIterator<Item = Component>,
    ) -> Result<Self, Error> {
        let custom_id = <_ as Into<String>>::into(custom_id).into_boxed_str();

        if custom_id.chars().count() > COMPONENT_CUSTOM_ID_LENGTH {
            return Err(Error::InvalidCustomId(custom_id));
        }

        let title = <_ as Into<String>>::into(title).into_boxed_str();
        let components = components.into_iter().collect();

        Ok(Self { custom_id, title, components })
    }
}

/// Builds a modal's basic data.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ModalDataBuilder {
    /// The modal's custom identifier.
    custom_id: String,
    /// The modal's title.
    title: String,
    /// The modal's components.
    components: Vec<Component>,
}

impl ModalDataBuilder {
    /// Creates a new [`ModalDataBuilder`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the custom identifier or title are invalid.
    pub fn new(custom_id: impl Into<String>, title: impl Into<String>) -> Result<Self, Error> {
        let custom_id: String = custom_id.into();

        if custom_id.chars().count() > COMPONENT_CUSTOM_ID_LENGTH {
            return Err(Error::InvalidCustomId(custom_id.into_boxed_str()));
        }

        let title: String = title.into();

        if title.chars().count() > MODAL_TITLE_LENGTH {
            return Err(Error::InvalidTitle(title.into_boxed_str()));
        }

        Ok(Self { custom_id, title, components: Vec::with_capacity(MODAL_INPUT_COUNT) })
    }

    /// Adds the given input to the modal.
    ///
    /// # Errors
    ///
    /// This function will return an error if the input is invalid or the maximum number has been reached.
    pub fn input(&mut self, input: impl Into<TextInput>) -> Result<(), Error> {
        if self.components.len() >= MODAL_INPUT_COUNT {
            return Err(Error::MaximumInputs);
        }

        let input = Component::TextInput(input.into());

        self.components.push(ActionRow { components: vec![input] }.into());

        Ok(())
    }

    /// Builds the modal data.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data is invalid.
    pub fn build(self) -> Result<ModalData, Error> {
        ModalData::new(self.custom_id, self.title, self.components)
    }
}
