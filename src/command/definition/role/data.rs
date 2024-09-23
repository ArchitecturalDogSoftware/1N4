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

use anyhow::Result;
use ina_macro::Stored;
use ina_storage::format::{Compress, Messagepack};
use serde::{Deserialize, Serialize};
use twilight_model::channel::message::Component;
use twilight_model::channel::message::component::{Button, ButtonStyle};
use twilight_model::id::Id;
use twilight_model::id::marker::{GuildMarker, RoleMarker, UserMarker};

use crate::command::registry::CommandEntry;
use crate::utility::traits::convert::AsEmoji;
use crate::utility::types::builder::{ActionRowBuilder, ButtonBuilder};
use crate::utility::types::id::CustomId;

/// A role selector entry.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selector {
    /// The role's identifier.
    pub id: Id<RoleMarker>,
    /// The selector's icon.
    pub icon: Box<str>,
    /// The selector's name.
    pub name: Box<str>,
}

impl Selector {
    /// Builds the selector entry into a button.
    ///
    /// # Errors
    ///
    /// This function will return an error if the button could not be created.
    pub fn build(&self, entry: &CommandEntry, kind: &'static str, disabled: bool) -> Result<Button> {
        let style = if kind == super::component::remove::NAME { ButtonStyle::Danger } else { ButtonStyle::Secondary };
        let custom_id = CustomId::<Box<str>>::new(entry.name, kind)?;

        Ok(ButtonBuilder::new(style)
            .custom_id(custom_id.with(self.id.to_string())?)?
            .disabled(disabled)
            .emoji(self.icon.as_emoji()?)?
            .label(self.name.as_ref())?
            .build())
    }
}

/// A list of role selector entries.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Stored)]
#[data_format(kind = Compress<Messagepack>, from = Compress::new_fast(Messagepack))]
#[data_path(fmt = "role/{}/{}", args = [Id<GuildMarker>, Id<UserMarker>], from = [guild_id, user_id])]
pub struct SelectorList {
    /// The user identifier.
    pub user_id: Id<UserMarker>,
    /// The guild identifier.
    pub guild_id: Id<GuildMarker>,
    /// The inner list of selectors.
    pub inner: Vec<Selector>,
}

impl SelectorList {
    /// Creates a new [`SelectorList`].
    pub const fn new(guild_id: Id<GuildMarker>, user_id: Id<UserMarker>) -> Self {
        Self { user_id, guild_id, inner: Vec::new() }
    }

    /// Builds the selector entry list into a list of components.
    ///
    /// # Errors
    ///
    /// This function will return an error if a button could not be created.
    pub fn build(&self, entry: &CommandEntry, kind: &'static str, disabled: bool) -> Result<Box<[Component]>> {
        let action_row_count = self.inner.len().div_ceil(5).min(5);
        let mut action_rows = Vec::<Component>::with_capacity(action_row_count);
        let mut action_row = ActionRowBuilder::new();

        for (index, selector) in self.inner.iter().enumerate() {
            if index != 0 && index % 5 == 0 {
                action_rows.push(action_row.build().into());

                action_row = ActionRowBuilder::new();
            }

            let button = selector.build(entry, kind, disabled)?;

            action_row = action_row.component(button)?;
        }

        let action_row = action_row.build();

        if !action_row.components.is_empty() {
            action_rows.push(action_row.into());
        }

        Ok(action_rows.into_boxed_slice())
    }
}
