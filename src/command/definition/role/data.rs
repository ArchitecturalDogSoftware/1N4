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

use std::ops::{Deref, DerefMut};

use anyhow::Result;
use ina_macro::Stored;
use ina_storage::format::{Compress, Messagepack};
use serde::{Deserialize, Serialize};
use twilight_model::channel::message::component::{Button, ButtonStyle};
use twilight_model::id::marker::{GuildMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;

use crate::command::registry::CommandEntry;
use crate::utility::traits::convert::AsEmoji;
use crate::utility::types::id::CustomId;

/// A role selector entry.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The role's identifier.
    pub id: Id<RoleMarker>,
    /// The selector's icon.
    pub icon: Box<str>,
    /// The selector's name.
    pub name: Box<str>,
}

impl Entry {
    /// Builds the selector entry into a button.
    ///
    /// # Errors
    ///
    /// This function will return an error if the button could not be created.
    pub fn build(&self, entry: &CommandEntry, disabled: bool) -> Result<Button> {
        let mut custom_id = CustomId::<Box<str>>::new(entry.name, super::SELECT_COMPONENT_NAME)?;

        custom_id.push(self.id.to_string())?;

        let custom_id = Some(custom_id.to_string());
        let emoji = Some(self.icon.as_emoji()?);
        let label = Some(self.name.to_string());

        Ok(Button { custom_id, disabled, emoji, label, style: ButtonStyle::Secondary, url: None })
    }
}

/// A list of role selector entries.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Stored)]
#[data_format(Compress<Messagepack>)]
#[data_path(fmt = "role/{}/{}", args = [Id<GuildMarker>, Id<UserMarker>], from = [guild_id, user_id])]
pub struct SelectorList {
    /// The user identifier.
    user_id: Id<UserMarker>,
    /// The guild identifier.
    guild_id: Id<GuildMarker>,
    /// The inner list of selectors.
    inner: Vec<Entry>,
}

impl Deref for SelectorList {
    type Target = Vec<Entry>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SelectorList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
