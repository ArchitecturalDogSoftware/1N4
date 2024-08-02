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
use std::future::Future;

use anyhow::{bail, Result};
use ina_localization::{localize, Locale, OwnedTranslation};
use time::macros::datetime;
use time::{Duration, OffsetDateTime};
use twilight_cache_inmemory::model::{CachedGuild, CachedMember};
use twilight_model::application::interaction::{Interaction, InteractionType};
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::channel::message::EmojiReactionType;
use twilight_model::guild::{Guild, Member, PartialMember};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;
use twilight_model::user::{CurrentUser, CurrentUserGuild, User};
use twilight_util::builder::embed::{EmbedAuthorBuilder, ImageSource};

/// Fallibly converts a reference to the implementing type into an embed author.
pub trait AsEmbedAuthor {
    /// The error that may be returned when converting.
    type Error;

    /// Fallibly converts this value into an embed author.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_embed_author(&self) -> Result<EmbedAuthor, Self::Error>;
}

impl AsEmbedAuthor for CachedGuild {
    type Error = anyhow::Error;

    #[inline]
    fn as_embed_author(&self) -> Result<EmbedAuthor, Self::Error> {
        Ok(EmbedAuthorBuilder::new(self.name()).icon_url(self.as_icon_source()?).build())
    }
}

impl AsEmbedAuthor for CurrentUser {
    type Error = anyhow::Error;

    #[inline]
    fn as_embed_author(&self) -> Result<EmbedAuthor, Self::Error> {
        Ok(EmbedAuthorBuilder::new(&self.name).icon_url(self.as_icon_source()?).build())
    }
}

impl AsEmbedAuthor for CurrentUserGuild {
    type Error = anyhow::Error;

    #[inline]
    fn as_embed_author(&self) -> Result<EmbedAuthor, Self::Error> {
        Ok(EmbedAuthorBuilder::new(&self.name).icon_url(self.as_icon_source()?).build())
    }
}

impl AsEmbedAuthor for Guild {
    type Error = anyhow::Error;

    #[inline]
    fn as_embed_author(&self) -> Result<EmbedAuthor, Self::Error> {
        Ok(EmbedAuthorBuilder::new(&self.name).icon_url(self.as_icon_source()?).build())
    }
}

impl AsEmbedAuthor for User {
    type Error = anyhow::Error;

    #[inline]
    fn as_embed_author(&self) -> Result<EmbedAuthor, Self::Error> {
        Ok(EmbedAuthorBuilder::new(&self.name).icon_url(self.as_icon_source()?).build())
    }
}

/// Fallibly converts a reference to the implementing type into an embed author.
pub trait AsEmbedAuthorWith<T> {
    /// The error that may be returned when converting.
    type Error;

    /// Fallibly converts this value into an embed author.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_embed_author_with(&self, value: T) -> Result<EmbedAuthor, Self::Error>;
}

impl<T: AsEmbedAuthor> AsEmbedAuthorWith<()> for T {
    type Error = <T as AsEmbedAuthor>::Error;

    #[inline]
    fn as_embed_author_with(&self, (): ()) -> Result<EmbedAuthor, Self::Error> {
        <T as AsEmbedAuthor>::as_embed_author(self)
    }
}

impl AsEmbedAuthorWith<Id<GuildMarker>> for Member {
    type Error = anyhow::Error;

    fn as_embed_author_with(&self, guild_id: Id<GuildMarker>) -> Result<EmbedAuthor, Self::Error> {
        let name = self.nick.as_deref().unwrap_or(&self.user.name);

        Ok(EmbedAuthorBuilder::new(name).icon_url(self.as_icon_source_with(guild_id)?).build())
    }
}

impl AsEmbedAuthorWith<Id<GuildMarker>> for PartialMember {
    type Error = anyhow::Error;

    fn as_embed_author_with(&self, guild_id: Id<GuildMarker>) -> Result<EmbedAuthor, Self::Error> {
        let Some(name) = self.nick.as_deref().or_else(|| self.user.as_ref().map(|v| &(*v.name))) else {
            bail!("missing member name");
        };

        Ok(EmbedAuthorBuilder::new(name).icon_url(self.as_icon_source_with(guild_id)?).build())
    }
}

/// Extends an emoji reaction type.
pub trait EmojiReactionTypeExt: Sized {
    /// The error that may be returned when creating the emoji.
    type Error;

    /// Attempts to parse the given string.
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn try_parse<T: Display>(value: T) -> Result<Self, Self::Error>;
}

impl EmojiReactionTypeExt for EmojiReactionType {
    type Error = anyhow::Error;

    fn try_parse<T: Display>(value: T) -> Result<Self, Self::Error> {
        let string = value.to_string();

        match string.chars().count() {
            0 => bail!("expected a non-empty display string"),
            1 => return Ok(Self::Unicode { name: string }),
            _ => {}
        }

        if !string.starts_with('<') {
            return Ok(Self::Unicode { name: string });
        }
        if !string.ends_with('>') {
            bail!("invalid emoji format");
        }

        let inner = string.trim_matches(['<', '>']);
        let mut split = inner.split(':');

        let animated = match split.next() {
            Some("a") => true,
            Some("") => false,
            _ => bail!("invalid emoji format"),
        };
        let Some(name) = split.next() else {
            bail!("missing emoji name");
        };
        let Some(id) = split.next().map(str::parse).transpose()? else {
            bail!("missing emoji identifier");
        };

        Ok(Self::Custom { animated, id, name: Some(name.to_string()) })
    }
}

/// A value with an associated icon.
pub trait IconHolder {
    /// The error that may be returned when creating the image.
    type Error;

    /// Fallibly returns this value's icon image source.
    ///
    /// # Errors
    ///
    /// This function will return an error if the source cannot be created.
    fn as_icon_source(&self) -> Result<ImageSource, Self::Error>;
}

impl IconHolder for CachedGuild {
    type Error = anyhow::Error;

    fn as_icon_source(&self) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.icon() else { bail!("missing icon hash") };
        let extension = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{}/icons/{}/{hash}.{extension}", Self::DISCORD_CDN_URL, self.id());

        ImageSource::url(url).map_err(Into::into)
    }
}

impl IconHolder for CurrentUser {
    type Error = anyhow::Error;

    fn as_icon_source(&self) -> Result<ImageSource, Self::Error> {
        let Some(ref hash) = self.avatar else { bail!("missing avatar hash") };
        let extension = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{}/avatars/{}/{hash}.{extension}", Self::DISCORD_CDN_URL, self.id);

        ImageSource::url(url).map_err(Into::into)
    }
}

impl IconHolder for CurrentUserGuild {
    type Error = anyhow::Error;

    fn as_icon_source(&self) -> Result<ImageSource, Self::Error> {
        let Some(ref hash) = self.icon else { bail!("missing icon hash") };
        let extension = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{}/icons/{}/{hash}.{extension}", Self::DISCORD_CDN_URL, self.id);

        ImageSource::url(url).map_err(Into::into)
    }
}

impl IconHolder for EmojiReactionType {
    type Error = anyhow::Error;

    fn as_icon_source(&self) -> Result<ImageSource, Self::Error> {
        let url = match self {
            Self::Custom { animated, id, .. } => {
                let extension = if *animated { "gif" } else { "png" };

                format!("{}/emojis/{id}.{extension}", Self::DISCORD_CDN_URL)
            }
            Self::Unicode { ref name } => {
                // Each file is encoded as hex numbers separated by hyphens. Some examples:
                // - '.../1f3f3-fe0f-200d-26a7-fe0f.png' for the transgender flag.
                // - '.../1f577-fe0f-fe0f' for the spider emoji.
                // - '.../1f578-fe0f-fe0f-fe0f' for the cobweb emoji.
                // See also: spiders 🕷️🕸️.
                let id = name.chars().map(|c| format!("{:x}", c as u32));

                format!("{}/{}.png", Self::TWEMOJI_CDN_URL, id.collect::<Box<[_]>>().join("-"))
            }
        };

        ImageSource::url(url).map_err(Into::into)
    }
}

impl IconHolder for Guild {
    type Error = anyhow::Error;

    fn as_icon_source(&self) -> Result<ImageSource, Self::Error> {
        let Some(ref hash) = self.icon else { bail!("missing icon hash") };
        let extension = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{}/icons/{}/{hash}.{extension}", Self::DISCORD_CDN_URL, self.id);

        ImageSource::url(url).map_err(Into::into)
    }
}

impl IconHolder for User {
    type Error = anyhow::Error;

    fn as_icon_source(&self) -> Result<ImageSource, Self::Error> {
        let Some(ref hash) = self.avatar else { bail!("missing avatar hash") };
        let extension = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{}/avatars/{}/{hash}.{extension}", Self::DISCORD_CDN_URL, self.id);

        ImageSource::url(url).map_err(Into::into)
    }
}

/// A value with an associated icon, computed using a value of type `<T>`.
pub trait IconHolderFrom<T> {
    /// The base Discord CDN URL.
    const DISCORD_CDN_URL: &str = "https://cdn.discordapp.com";
    /// The base twemoji CDN URL.
    const TWEMOJI_CDN_URL: &str = "https://raw.githubusercontent.com/discord/twemoji/main/assets/72x72";

    /// The error that may be returned when creating the image.
    type Error;

    /// Fallibly returns this value's icon image source, computed using the given value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the source cannot be created.
    fn as_icon_source_with(&self, value: T) -> Result<ImageSource, Self::Error>;
}

impl<T: IconHolder> IconHolderFrom<()> for T {
    type Error = <T as IconHolder>::Error;

    #[inline]
    fn as_icon_source_with(&self, (): ()) -> Result<ImageSource, Self::Error> {
        <T as IconHolder>::as_icon_source(self)
    }
}

impl IconHolderFrom<Id<GuildMarker>> for CachedMember {
    type Error = anyhow::Error;

    fn as_icon_source_with(&self, guild_id: Id<GuildMarker>) -> Result<ImageSource, Self::Error> {
        let Some(ref hash) = self.avatar() else { bail!("missing avatar hash") };
        let extension = if hash.is_animated() { "gif" } else { "png" };

        ImageSource::url(format!(
            "{}/guilds/{guild_id}/users/{}/avatars/{hash}.{extension}",
            Self::DISCORD_CDN_URL,
            self.user_id()
        ))
        .map_err(Into::into)
    }
}

impl IconHolderFrom<Id<GuildMarker>> for Member {
    type Error = anyhow::Error;

    fn as_icon_source_with(&self, guild_id: Id<GuildMarker>) -> Result<ImageSource, Self::Error> {
        let Some(ref hash) = self.avatar else { bail!("missing avatar hash") };
        let extension = if hash.is_animated() { "gif" } else { "png" };

        ImageSource::url(format!(
            "{}/guilds/{guild_id}/users/{}/avatars/{hash}.{extension}",
            Self::DISCORD_CDN_URL,
            self.user.id
        ))
        .map_err(Into::into)
    }
}

impl IconHolderFrom<Id<GuildMarker>> for PartialMember {
    type Error = anyhow::Error;

    fn as_icon_source_with(&self, guild_id: Id<GuildMarker>) -> Result<ImageSource, Self::Error> {
        let Some(ref user) = self.user else { bail!("missing user data") };
        let Some(ref hash) = self.avatar else { bail!("missing avatar hash") };
        let extension = if hash.is_animated() { "gif" } else { "png" };

        ImageSource::url(format!(
            "{}/guilds/{guild_id}/users/{}/avatars/{hash}.{extension}",
            Self::DISCORD_CDN_URL,
            user.id
        ))
        .map_err(Into::into)
    }
}

/// Provides extension methods to identifier structs.
pub trait IdExt {
    /// Returns the value's creation date and time based off of its identifier.
    fn creation_date(&self) -> OffsetDateTime;
}

impl<T> IdExt for Id<T> {
    #[allow(clippy::cast_possible_wrap)]
    fn creation_date(&self) -> OffsetDateTime {
        const DISCORD_EPOCH: OffsetDateTime = datetime!(2015-01-01 00:00:00 UTC);

        let milliseconds = (self.get() >> 22).min(i64::MAX as u64) as i64;

        DISCORD_EPOCH.saturating_add(Duration::milliseconds(milliseconds))
    }
}

/// Extends an interaction.
pub trait InteractionExt {
    /// Returns a display implementation for logging purposes.
    fn display(&self) -> impl Display;
}

impl InteractionExt for Interaction {
    fn display(&self) -> impl Display {
        struct Label<'ev>(&'ev Interaction);

        impl<'ev> Label<'ev> {
            /// Returns a string representing the interaction's type.
            const fn kind(&self) -> &'static str {
                match self.0.kind {
                    InteractionType::Ping => "ping",
                    InteractionType::ApplicationCommand => "command",
                    InteractionType::MessageComponent => "component",
                    InteractionType::ApplicationCommandAutocomplete => "autocomplete",
                    InteractionType::ModalSubmit => "modal",
                    _ => "unknown",
                }
            }
        }

        impl Display for Label<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let kind = self.kind();

                if let Some(user_id) = self.0.author_id() {
                    write!(f, "<{kind}:{}:{user_id}>", self.0.id)
                } else {
                    write!(f, "<{kind}:{}>", self.0.id)
                }
            }
        }

        Label(self)
    }
}

/// A value that can be localized.
pub trait Localizable {
    /// Returns this value's localization category.
    fn category(&self) -> &str;

    /// Returns this value's localization key.
    fn key(&self) -> &str;

    /// Attempts to localize this value, returning the translation.
    ///
    /// # Errors
    ///
    /// This function will return an error if localization fails.
    #[inline]
    fn localized(
        &self,
        locale: Option<Locale>,
    ) -> impl Future<Output = Result<OwnedTranslation<ina_localization::thread::Inner>>> + Send
    where
        Self: Send + Sync,
    {
        async move { localize!(async(try in locale) self.category(), self.key()).await.map_err(Into::into) }
    }

    /// Attempts to localize this value, returning the translation.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called from within an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if localization fails.
    #[inline]
    fn blocking_localized(&self, locale: Option<Locale>) -> Result<OwnedTranslation<ina_localization::thread::Inner>> {
        localize!((try in locale) self.category(), self.key()).map_err(Into::into)
    }
}

/// A value that has a preferred locale.
pub trait LocaleHolder {
    /// Returns this value's preferred locale, if set.
    fn preferred_locale(&self) -> Option<Locale> {
        None
    }
}

impl<T: LocaleHolder> LocaleHolder for Option<T> {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> {
        self.as_ref().and_then(LocaleHolder::preferred_locale)
    }
}

impl LocaleHolder for CachedGuild {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> {
        self.preferred_locale().parse().ok()
    }
}

impl LocaleHolder for CurrentUser {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> {
        self.locale.as_deref().and_then(|s| s.parse().ok())
    }
}

impl LocaleHolder for Guild {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> {
        self.preferred_locale.parse().ok()
    }
}

impl LocaleHolder for Interaction {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> {
        self.locale.as_deref().and_then(|s| s.parse().ok())
    }
}

impl LocaleHolder for Member {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> {
        self.user.preferred_locale()
    }
}

impl LocaleHolder for PartialMember {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> {
        self.user.preferred_locale()
    }
}

impl LocaleHolder for User {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> {
        self.locale.as_deref().and_then(|s| s.parse().ok())
    }
}
