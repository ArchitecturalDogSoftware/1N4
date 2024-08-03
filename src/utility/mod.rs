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

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use twilight_model::channel::message::component::{ActionRow, TextInput};
use twilight_model::channel::message::Component;

/// Provides getters for client secrets.
pub mod secret;
/// Provides commonly-used trait definitions and blanket implementations.
pub mod traits;

/// The base Discord CDN URL.
pub const DISCORD_CDN_URL: &str = "https://cdn.discordapp.com";
/// The base twemoji CDN URL.
pub const TWEMOJI_CDN_URL: &str = "https://raw.githubusercontent.com/discord/twemoji/main/assets/72x72";

/// A modal's data.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModalData {
    /// The modal's custom identifier.
    pub custom_id: String,
    /// The modal's tite.
    pub title: String,
    /// The modal's component list.
    pub components: Vec<Component>,
}

impl ModalData {
    /// The maximum number of permitted inputs.
    const MAX_INPUTS: usize = 5;

    /// Creates a new [`ModalData`].
    pub fn new(custom_id: impl AsRef<str>, title: impl AsRef<str>) -> Self {
        let components = Vec::with_capacity(Self::MAX_INPUTS);

        Self { custom_id: custom_id.as_ref().to_string(), title: title.as_ref().to_string(), components }
    }

    /// Adds an input onto this modal.
    ///
    /// # Errors
    ///
    /// This function will return an error if the input could not be added.
    pub fn input(&mut self, input: impl Into<TextInput>) -> Result<()> {
        ensure!(self.components.len() < Self::MAX_INPUTS, "a maximum of {} components are permitted", Self::MAX_INPUTS);

        let input = Component::TextInput(input.into());

        self.components.push(ActionRow { components: vec![input] }.into());

        Ok(())
    }
}
