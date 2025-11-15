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
use twilight_validate::component::COMPONENT_CUSTOM_ID_LENGTH;

use crate::utility::types::builder::ValidatedBuilder;

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
    InvalidCustomId(String),
    /// An invalid title was provided.
    #[error("an invalid title was provided: '{0}'")]
    InvalidTitle(String),
    /// Returned when attempting to add more than the allowed number of inputs.
    #[error("a maximum of {MODAL_INPUT_COUNT} inputs is permitted")]
    MaximumInputs,
    /// Returned when no inputs are provided.
    #[error("a minimum of 1 input is required")]
    MissingInput,
}

/// A modal.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modal {
    /// The modal's title.
    pub title: String,
    /// The modal's custom identifier.
    pub custom_id: String,
    /// The modal's components.
    pub components: Vec<Component>,
}

/// A modal builder.
#[must_use]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ModalBuilder {
    /// The inner modal.
    inner: Modal,
}

impl ModalBuilder {
    /// Creates a new [`ModalBuilder`].
    pub fn new(title: impl Into<String>, custom_id: impl Into<String>) -> Self {
        Self { inner: Modal { title: title.into(), custom_id: custom_id.into(), components: Vec::with_capacity(5) } }
    }

    /// Adds the given component to the modal.
    pub fn component(mut self, component: impl Into<Component>) -> Self {
        self.inner.components.push(component.into());
        self
    }

    /// Builds the finished [`Modal`].
    #[must_use]
    pub fn build(self) -> Modal {
        self.inner
    }
}

impl ValidatedBuilder for ModalBuilder {
    type Error = Error;
    type Output = Modal;

    fn validate(inner: &Self::Output) -> Result<(), Self::Error> {
        if inner.title.len() > MODAL_TITLE_LENGTH {
            Err(Error::InvalidTitle(inner.title.clone()))
        } else if inner.custom_id.len() > COMPONENT_CUSTOM_ID_LENGTH {
            Err(Error::InvalidCustomId(inner.custom_id.clone()))
        } else if inner.components.len() > MODAL_INPUT_COUNT {
            Err(Error::MaximumInputs)
        } else {
            Ok(())
        }
    }

    fn try_build(self) -> Result<Self::Output, Self::Error> {
        <Self as ValidatedBuilder>::validate(&self.inner).map(|()| self.inner)
    }
}
