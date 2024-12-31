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

use std::fmt::Display;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use twilight_http::request::channel::message::UpdateMessage;
use twilight_model::channel::Message;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker};

use crate::client::api::ApiRef;

/// A reference to an existing message.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Anchor {
    /// The guild identifier.
    pub guild_id: Option<Id<GuildMarker>>,
    /// The channel identifier.
    pub channel_id: Id<ChannelMarker>,
    /// The message identifier.
    pub message_id: Id<MessageMarker>,
}

impl Anchor {
    /// Creates a new guild channel [`Anchor`].
    #[must_use]
    pub const fn new_guild(
        guild_id: Id<GuildMarker>,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
    ) -> Self {
        Self { guild_id: Some(guild_id), channel_id, message_id }
    }

    /// Creates a new private channel [`Anchor`].
    #[must_use]
    pub const fn new_private(channel_id: Id<ChannelMarker>, message_id: Id<MessageMarker>) -> Self {
        Self { guild_id: None, channel_id, message_id }
    }

    /// Returns a display implementation for this [`Anchor`]'s link.
    pub const fn display_link(&self) -> AnchorLinkDisplay {
        AnchorLinkDisplay(self)
    }

    /// Returns the associated message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be fetched.
    pub async fn message(&mut self, api: ApiRef<'_>) -> Result<Message> {
        let Self { channel_id, message_id, .. } = self;

        Ok(api.client.message(*channel_id, *message_id).await?.model().await?)
    }

    /// Returns a message update future builder for the associated message.
    pub fn update<'ar>(&self, api: ApiRef<'ar>) -> UpdateMessage<'ar> {
        let Self { channel_id, message_id, .. } = self;

        api.client.update_message(*channel_id, *message_id)
    }

    /// Deletes the associated message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be deleted.
    pub async fn delete(&mut self, api: ApiRef<'_>) -> Result<()> {
        let Self { channel_id, message_id, .. } = self;

        api.client.delete_message(*channel_id, *message_id).await?;

        Ok(())
    }

    /// Deletes the associated message if it exists.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be deleted.
    pub async fn delete_if_present(&mut self, api: ApiRef<'_>) -> Result<()> {
        if self.message(api).await.is_ok() {
            self.delete(api).await?;
        }

        Ok(())
    }
}

impl From<Message> for Anchor {
    fn from(value: Message) -> Self {
        <Self as From<&Message>>::from(&value)
    }
}

impl From<&Message> for Anchor {
    fn from(value: &Message) -> Self {
        let &Message { channel_id, guild_id, id, .. } = value;

        Self { guild_id, channel_id, message_id: id }
    }
}

/// Display's an [`Anchor`] as a clickable link.
#[must_use = "this value does nothing unless displayed"]
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct AnchorLinkDisplay<'ak>(&'ak Anchor);

impl Display for AnchorLinkDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const BASE_URL: &str = "https://discord.com/channels";

        let Self(Anchor { channel_id, message_id, .. }) = self;

        if let Some(guild_id) = self.0.guild_id {
            write!(f, "{BASE_URL}/{guild_id}/{channel_id}/{message_id}")
        } else {
            write!(f, "{BASE_URL}/@me/{channel_id}/{message_id}")
        }
    }
}
