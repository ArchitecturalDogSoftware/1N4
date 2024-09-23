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

use std::convert::identity;
use std::fmt::Display;
use std::num::NonZeroU16;

use time::macros::datetime;
use time::{Duration, OffsetDateTime};
use twilight_cache_inmemory::model::{CachedGuild, CachedMember};
use twilight_model::application::interaction::{Interaction, InteractionType};
use twilight_model::gateway::payload::incoming::invite_create::PartialUser;
use twilight_model::guild::template::TemplateGuild;
use twilight_model::guild::{Guild, GuildInfo, GuildPreview, Member, PartialGuild, PartialMember};
use twilight_model::id::Id;
use twilight_model::id::marker::{InteractionMarker, UserMarker};
use twilight_model::user::{CurrentUser, CurrentUserGuild, User};
use twilight_model::util::ImageHash;

/// Extends an [`Id<T>`] or other identifier-like types.
pub trait IdExt<T> {
    /// Returns the identifier's creation date.
    fn creation_date(&self) -> OffsetDateTime;
}

impl<T> IdExt<T> for Id<T> {
    fn creation_date(&self) -> OffsetDateTime {
        const DISCORD_EPOCH: OffsetDateTime = datetime!(2015-01-01 00:00:00 UTC);

        #[expect(clippy::cast_possible_wrap, reason = "this will never feasibly be high enough to wrap")]
        let milliseconds = (self.get() >> 22).min(i64::MAX as u64) as i64;

        DISCORD_EPOCH.saturating_add(Duration::milliseconds(milliseconds))
    }
}

/// Extends an [`Interaction`] or other interaction-like types.
pub trait InteractionExt {
    /// Returns a display representation of this interaction's type.
    fn display_label(&self) -> InteractionLabelDisplay;
}

/// Displays an interaction label.
#[must_use = "this value does nothing unless displayed"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct InteractionLabelDisplay<'ev> {
    /// The interaction identifier.
    id: Id<InteractionMarker>,
    /// The interaction type string.
    kind: &'ev str,
    /// The user identifier.
    user_id: Option<Id<UserMarker>>,
}

impl<'ev> Display for InteractionLabelDisplay<'ev> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(user_id) = self.user_id {
            write!(f, "<{}:{}:{user_id}>", self.kind, self.id)
        } else {
            write!(f, "<{}:{}>", self.kind, self.id)
        }
    }
}

impl InteractionExt for Interaction {
    fn display_label(&self) -> InteractionLabelDisplay {
        let kind = match self.kind {
            InteractionType::Ping => "ping",
            InteractionType::ApplicationCommand => "command",
            InteractionType::MessageComponent => "component",
            InteractionType::ApplicationCommandAutocomplete => "autocomplete",
            InteractionType::ModalSubmit => "modal",
            _ => "unknown",
        };

        InteractionLabelDisplay { id: self.id, kind, user_id: self.author_id() }
    }
}

/// Extends a [`Guild`] or other guild-like types.
pub trait GuildExt {
    /// Returns a display representation of this guild's name.
    fn name(&self) -> &str;

    /// Returns the guild's icon hash.
    fn icon_hash(&self) -> Option<&ImageHash>;
}

macro_rules! guild_ext_impl {
    ($($type:ty)*) => {$(
        impl GuildExt for $type {
            fn name(&self) -> &str {
                &self.name
            }

            fn icon_hash(&self) -> Option<&ImageHash> {
                self.icon.as_ref()
            }
        }
    )*};
}

guild_ext_impl!(CurrentUserGuild Guild GuildInfo GuildPreview PartialGuild);

impl GuildExt for CachedGuild {
    fn name(&self) -> &str {
        self.name()
    }

    fn icon_hash(&self) -> Option<&ImageHash> {
        self.icon()
    }
}

impl GuildExt for TemplateGuild {
    fn name(&self) -> &str {
        &self.name
    }

    fn icon_hash(&self) -> Option<&ImageHash> {
        self.icon_hash.as_ref()
    }
}

/// Extends a [`User`] or other user-like types.
pub trait UserExt {
    /// Returns a display implementation of this user's name.
    fn display_name(&self) -> UserNameDisplay;

    /// Returns a display implementation of this user's account tag.
    fn display_tag(&self) -> UserTagDisplay;

    /// Returns the user's icon hash.
    fn icon_hash(&self) -> Option<ImageHash>;

    /// Returns the user's banner hash.
    fn banner_hash(&self) -> Option<ImageHash>;
}

/// Displays a user's name.
#[must_use = "this value does nothing unless displayed"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct UserNameDisplay<'us> {
    /// The user's guild nickname.
    nick: Option<&'us str>,
    /// The user's display name.
    name: Option<&'us str>,
    /// The user's account name.
    user: &'us str,
}

impl<'us> Display for UserNameDisplay<'us> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.nick.map_or(self.name.map_or(self.user, identity), identity))
    }
}

/// Displays a user's account tag.
#[must_use = "this value does nothing unless displayed"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct UserTagDisplay<'us> {
    /// The user's username.
    user: &'us str,
    /// The user's discriminator tag.
    tag: Option<NonZeroU16>,
}

impl<'us> Display for UserTagDisplay<'us> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(discriminator) = self.tag {
            write!(f, "{}#{discriminator:04}", self.user)
        } else {
            write!(f, "@{}", self.user)
        }
    }
}

impl UserExt for CachedMember {
    fn display_name(&self) -> UserNameDisplay {
        UserNameDisplay { nick: self.nick(), name: None, user: "unknown" }
    }

    fn display_tag(&self) -> UserTagDisplay {
        UserTagDisplay { user: "unknown", tag: None }
    }

    fn icon_hash(&self) -> Option<ImageHash> {
        self.avatar()
    }

    fn banner_hash(&self) -> Option<ImageHash> {
        None
    }
}

impl UserExt for CurrentUser {
    fn display_name(&self) -> UserNameDisplay {
        UserNameDisplay { nick: None, name: None, user: &self.name }
    }

    fn display_tag(&self) -> UserTagDisplay {
        UserTagDisplay { user: &self.name, tag: NonZeroU16::new(self.discriminator) }
    }

    fn icon_hash(&self) -> Option<ImageHash> {
        self.avatar
    }

    fn banner_hash(&self) -> Option<ImageHash> {
        self.banner
    }
}

impl UserExt for Member {
    fn display_name(&self) -> UserNameDisplay {
        UserNameDisplay { nick: self.nick.as_deref(), name: self.user.global_name.as_deref(), user: &self.user.name }
    }

    fn display_tag(&self) -> UserTagDisplay {
        UserTagDisplay { user: &self.user.name, tag: NonZeroU16::new(self.user.discriminator) }
    }

    fn icon_hash(&self) -> Option<ImageHash> {
        self.avatar.or(self.user.avatar)
    }

    fn banner_hash(&self) -> Option<ImageHash> {
        self.user.banner
    }
}

impl UserExt for PartialMember {
    fn display_name(&self) -> UserNameDisplay {
        UserNameDisplay {
            nick: self.nick.as_deref(),
            name: self.user.as_ref().and_then(|u| u.global_name.as_deref()),
            user: self.user.as_ref().map_or("unknown", |u| &(*u.name)),
        }
    }

    fn display_tag(&self) -> UserTagDisplay {
        UserTagDisplay {
            user: self.user.as_ref().map_or("unknown", |u| &(*u.name)),
            tag: self.user.as_ref().and_then(|u| NonZeroU16::new(u.discriminator)),
        }
    }

    fn icon_hash(&self) -> Option<ImageHash> {
        self.avatar.or_else(|| self.user.as_ref().and_then(|u| u.avatar))
    }

    fn banner_hash(&self) -> Option<ImageHash> {
        self.user.as_ref().and_then(|u| u.banner)
    }
}

impl UserExt for PartialUser {
    fn display_name(&self) -> UserNameDisplay {
        UserNameDisplay { nick: None, name: None, user: &self.username }
    }

    fn display_tag(&self) -> UserTagDisplay {
        UserTagDisplay { user: &self.username, tag: NonZeroU16::new(self.discriminator) }
    }

    fn icon_hash(&self) -> Option<ImageHash> {
        self.avatar
    }

    fn banner_hash(&self) -> Option<ImageHash> {
        None
    }
}

impl UserExt for User {
    fn display_name(&self) -> UserNameDisplay {
        UserNameDisplay { nick: None, name: self.global_name.as_deref(), user: &self.name }
    }

    fn display_tag(&self) -> UserTagDisplay {
        UserTagDisplay { user: &self.name, tag: NonZeroU16::new(self.discriminator) }
    }

    fn icon_hash(&self) -> Option<ImageHash> {
        self.avatar
    }

    fn banner_hash(&self) -> Option<ImageHash> {
        self.banner
    }
}
