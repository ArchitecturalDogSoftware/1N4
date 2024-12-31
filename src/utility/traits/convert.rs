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

use std::convert::Infallible;
use std::str::FromStr;

use anyhow::{bail, ensure};
use ina_localizing::locale::Locale;
use twilight_cache_inmemory::model::{CachedEmoji, CachedGuild, CachedMember, CachedMessage, CachedSticker};
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::channel::message::sticker::StickerFormatType;
use twilight_model::channel::message::{EmojiReactionType, Sticker};
use twilight_model::channel::{Attachment, Channel, Message};
use twilight_model::gateway::payload::incoming::invite_create::PartialUser;
use twilight_model::guild::template::TemplateGuild;
use twilight_model::guild::{Emoji, Guild, GuildInfo, GuildPreview, Member, PartialGuild, PartialMember};
use twilight_model::id::Id;
use twilight_model::id::marker::{
    AttachmentMarker, ChannelMarker, EmojiMarker, GuildMarker, InteractionMarker, MessageMarker, StickerMarker,
    UserMarker,
};
use twilight_model::user::{CurrentUser, CurrentUserGuild, User};
use twilight_util::builder::embed::{EmbedAuthorBuilder, ImageSource};

use super::extension::{GuildExt, UserExt};
use crate::utility::{DISCORD_CDN_URL, TWEMOJI_CDN_URL};

/// Converts the implementing type into an embed author.
pub trait AsEmbedAuthor {
    /// The error that may be returned when converting.
    type Error;

    /// Fallibly converts this value into an embed author builder.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_embed_author_builder(&self) -> Result<EmbedAuthorBuilder, Self::Error>;

    /// Fallibly converts this value into an embed author.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_embed_author(&self) -> Result<EmbedAuthor, Self::Error> {
        self.as_embed_author_builder().map(EmbedAuthorBuilder::build)
    }
}

/// Fallibly converts a guild into an embed author builder.
///
/// # Errors
///
/// This function will return an error if the value could not be converted.
fn guild_as_embed_author_builder<G: GuildExt + AsImageSource>(value: &G) -> Result<EmbedAuthorBuilder, G::Error> {
    let name = value.name();
    let icon = value.as_image_source()?;

    Ok(EmbedAuthorBuilder::new(name).icon_url(icon))
}

/// Fallibly converts a user into an embed author builder.
///
/// # Errors
///
/// This function will return an error if the value could not be converted.
fn user_as_embed_author_builder<U: UserExt + AsImageSource>(value: &U) -> Result<EmbedAuthorBuilder, U::Error> {
    let name = value.display_name().to_string();
    let icon = value.as_image_source()?;

    Ok(EmbedAuthorBuilder::new(name).icon_url(icon))
}

impl AsEmbedAuthor for CachedGuild {
    type Error = <Self as AsImageSource>::Error;

    fn as_embed_author_builder(&self) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::guild_as_embed_author_builder(self)
    }
}

impl AsEmbedAuthor for CurrentUser {
    type Error = <Self as AsImageSource>::Error;

    fn as_embed_author_builder(&self) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::user_as_embed_author_builder(self)
    }
}

impl AsEmbedAuthor for CurrentUserGuild {
    type Error = <Self as AsImageSource>::Error;

    fn as_embed_author_builder(&self) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::guild_as_embed_author_builder(self)
    }
}

impl AsEmbedAuthor for Guild {
    type Error = <Self as AsImageSource>::Error;

    fn as_embed_author_builder(&self) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::guild_as_embed_author_builder(self)
    }
}

impl AsEmbedAuthor for GuildInfo {
    type Error = <Self as AsImageSource>::Error;

    fn as_embed_author_builder(&self) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::guild_as_embed_author_builder(self)
    }
}

impl AsEmbedAuthor for GuildPreview {
    type Error = <Self as AsImageSource>::Error;

    fn as_embed_author_builder(&self) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::guild_as_embed_author_builder(self)
    }
}

impl AsEmbedAuthor for PartialGuild {
    type Error = <Self as AsImageSource>::Error;

    fn as_embed_author_builder(&self) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::guild_as_embed_author_builder(self)
    }
}

impl AsEmbedAuthor for User {
    type Error = <Self as AsImageSource>::Error;

    fn as_embed_author_builder(&self) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::user_as_embed_author_builder(self)
    }
}

/// Converts the implementing type into an embed author using the type `<T>` as an argument.
pub trait AsEmbedAuthorWith<T> {
    /// The error that may be returned when converting.
    type Error;

    /// Fallibly converts this value into an embed author builder with the given argument.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_embed_author_builder_with(&self, value: T) -> Result<EmbedAuthorBuilder, Self::Error>;

    /// Fallibly converts this value into an embed author with the given argument.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_embed_author_with(&self, value: T) -> Result<EmbedAuthor, Self::Error> {
        self.as_embed_author_builder_with(value).map(EmbedAuthorBuilder::build)
    }
}

/// Fallibly converts a user into an embed author builder.
///
/// # Errors
///
/// This function will return an error if the value could not be converted.
fn user_as_embed_author_builder_with<U: UserExt + AsImageSourceWith<T>, T>(
    value: &U,
    arguments: T,
) -> Result<EmbedAuthorBuilder, U::Error> {
    let name = value.display_name().to_string();
    let icon = value.as_image_source_with(arguments)?;

    Ok(EmbedAuthorBuilder::new(name).icon_url(icon))
}

impl AsEmbedAuthorWith<Id<GuildMarker>> for CachedMember {
    type Error = anyhow::Error;

    fn as_embed_author_builder_with(&self, value: Id<GuildMarker>) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::user_as_embed_author_builder_with(self, value)
    }
}

impl AsEmbedAuthorWith<Id<GuildMarker>> for Member {
    type Error = anyhow::Error;

    fn as_embed_author_builder_with(&self, value: Id<GuildMarker>) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::user_as_embed_author_builder_with(self, value)
    }
}

impl AsEmbedAuthorWith<Id<GuildMarker>> for PartialMember {
    type Error = anyhow::Error;

    fn as_embed_author_builder_with(&self, value: Id<GuildMarker>) -> Result<EmbedAuthorBuilder, Self::Error> {
        self::user_as_embed_author_builder_with(self, value)
    }
}

/// Converts the implementing type into an emoji.
pub trait AsEmoji {
    /// The error that may be returned when converting.
    type Error;

    /// Fallibly converts this value into an emoji.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_emoji(&self) -> Result<EmojiReactionType, Self::Error>;
}

impl AsEmoji for char {
    type Error = Infallible;

    fn as_emoji(&self) -> Result<EmojiReactionType, Self::Error> {
        Ok(EmojiReactionType::Unicode { name: self.to_string() })
    }
}

impl AsEmoji for str {
    type Error = anyhow::Error;

    fn as_emoji(&self) -> Result<EmojiReactionType, Self::Error> {
        ensure!(!self.is_empty(), "expected a non-empty string");

        if !self.starts_with('<') {
            return Ok(EmojiReactionType::Unicode { name: self.to_string() });
        }

        ensure!(self.ends_with('>'), "missing closing angle bracket");

        let inner = self.trim_matches(['<', '>']);
        let mut sections = inner.split(':');

        let animated = match sections.next() {
            Some(s @ ("" | "a")) => s == "a",
            Some(s) => bail!("invalid animated header: '{s}'"),
            None => bail!("missing animated header"),
        };

        let Some(name) = sections.next() else { bail!("missing emoji name") };

        ensure!(name.chars().count() > 1, "emoji name must be at least two characters");
        ensure!(
            name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "emoji name must be entirely alphanumeric including underscores"
        );

        let Some(id) = sections.next() else { bail!("missing emoji identifier") };

        let remaining = sections.collect::<Box<[_]>>();

        ensure!(remaining.is_empty(), "unexpected section(s) in emoji string: {remaining:?}");

        Ok(EmojiReactionType::Custom { animated, id: id.parse()?, name: Some(name.to_string()) })
    }
}

/// Converts the implementing type into an identifier.
pub trait AsId<T> {
    /// Converts this value into an identifier.
    fn as_id(&self) -> Id<T>;
}

macro_rules! as_id_impl {
    ($(
        $type:ty as $marker:ty $(= $field:ident)? $(=> $call:ident)? $(=> { $($body:tt)* })?;
    )*) => {$(
        impl AsId<$marker> for $type {
            fn as_id(&self) -> Id<$marker> {
                self.$($field)?$($call())?$($($body)*)?
            }
        }
    )*};
}

as_id_impl! {
    CachedEmoji as EmojiMarker => id;
    CachedGuild as GuildMarker => id;
    CachedMember as UserMarker => user_id;
    CachedMessage as MessageMarker => id;
    CachedSticker as StickerMarker => id;
    Attachment as AttachmentMarker = id;
    Channel as ChannelMarker = id;
    CurrentUser as UserMarker = id;
    CurrentUserGuild as GuildMarker = id;
    Emoji as EmojiMarker = id;
    Guild as GuildMarker = id;
    GuildInfo as GuildMarker = id;
    GuildPreview as GuildMarker = id;
    Interaction as InteractionMarker = id;
    Member as UserMarker => { user.id };
    Message as MessageMarker = id;
    PartialGuild as GuildMarker = id;
    PartialUser as UserMarker = id;
    Sticker as StickerMarker = id;
    User as UserMarker = id;
}

/// Converts the implementing type into an image source.
pub trait AsImageSource {
    /// The error that may be returned when converting.
    type Error;

    /// Fallibly converts this value into an image source.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_image_source(&self) -> Result<ImageSource, Self::Error>;
}

/// Converts the given guild into an image source.
///
/// # Errors
///
/// This function will return an error if the conversion fails.
fn guild_as_image_source<G: GuildExt + AsId<GuildMarker>>(value: &G) -> anyhow::Result<ImageSource> {
    let Some(hash) = value.icon_hash() else { bail!("missing icon hash") };
    let extension = if hash.is_animated() { "gif" } else { "png" };
    let url = format!("{DISCORD_CDN_URL}/icons/{}/{hash}.{extension}", value.as_id());

    ImageSource::url(url).map_err(Into::into)
}

/// Converts the given user into an image source.
///
/// # Errors
///
/// This function will return an error if the conversion fails.
fn user_as_image_source<U: UserExt + AsId<UserMarker>>(value: &U) -> anyhow::Result<ImageSource> {
    let Some(hash) = value.icon_hash() else { bail!("missing avatar hash") };
    let extension = if hash.is_animated() { "gif" } else { "png" };
    let url = format!("{DISCORD_CDN_URL}/avatars/{}/{hash}.{extension}", value.as_id());

    ImageSource::url(url).map_err(Into::into)
}

impl AsImageSource for CachedGuild {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        self::guild_as_image_source(self)
    }
}

impl AsImageSource for CurrentUser {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        self::user_as_image_source(self)
    }
}

impl AsImageSource for CurrentUserGuild {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        self::guild_as_image_source(self)
    }
}

impl AsImageSource for Emoji {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        let extension = if self.animated { "gif" } else { "png" };
        let url = format!("{DISCORD_CDN_URL}/emojis/{}.{extension}", self.id);

        ImageSource::url(url).map_err(Into::into)
    }
}

impl AsImageSource for EmojiReactionType {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        let url = match self {
            Self::Custom { animated, id, .. } => {
                let extension = if *animated { "gif" } else { "png" };

                format!("{DISCORD_CDN_URL}/emojis/{id}.{extension}")
            }
            Self::Unicode { name } => {
                // Each file is encoded as hex numbers separated by hyphens. Some examples:
                // - `.../1f3f3-fe0f-200d-26a7-fe0f.png` for the transgender flag.
                // - `.../1f577-fe0f-fe0f.png` for the spider emoji.
                // - `.../1f578-fe0f-fe0f-fe0f.png` for the cobweb emoji.
                // See also: spiders 🕷️🕸️.
                let id = name.chars().map(|c| format!("{:x}", c as u32));

                format!("{TWEMOJI_CDN_URL}/{}.png", id.collect::<Box<[_]>>().join("-"))
            }
        };

        ImageSource::url(url).map_err(Into::into)
    }
}

impl AsImageSource for Guild {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        self::guild_as_image_source(self)
    }
}

impl AsImageSource for GuildInfo {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        self::guild_as_image_source(self)
    }
}

impl AsImageSource for GuildPreview {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        self::guild_as_image_source(self)
    }
}

impl AsImageSource for PartialGuild {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        self::guild_as_image_source(self)
    }
}

impl AsImageSource for PartialUser {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        self::user_as_image_source(self)
    }
}

impl AsImageSource for Sticker {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        let extension = match self.format_type {
            StickerFormatType::Png | StickerFormatType::Apng => "png",
            StickerFormatType::Lottie => "json",
            StickerFormatType::Gif => "gif",
            _ => bail!("unknown sticker format"),
        };

        // Why do `.gif` stickers specifically use a different CDN??? This is stupid.
        let url = format!(
            "{}/stickers/{}.{extension}",
            if self.format_type == StickerFormatType::Gif { "https://media.discordapp.net" } else { DISCORD_CDN_URL },
            self.id
        );

        ImageSource::url(url).map_err(Into::into)
    }
}

impl AsImageSource for User {
    type Error = anyhow::Error;

    fn as_image_source(&self) -> Result<ImageSource, Self::Error> {
        self::user_as_image_source(self)
    }
}

/// Converts the implementing type into an image source using the type `<T>` as an argument.
pub trait AsImageSourceWith<T> {
    /// The error that may be returned when converting.
    type Error;

    /// Fallibly converts this value into an image source with the given argument.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_image_source_with(&self, value: T) -> Result<ImageSource, Self::Error>;
}

impl AsImageSourceWith<Id<GuildMarker>> for CachedMember {
    type Error = anyhow::Error;

    fn as_image_source_with(&self, value: Id<GuildMarker>) -> Result<ImageSource, Self::Error> {
        let Some(ref hash) = self.avatar() else { bail!("missing avatar hash") };
        let extension = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{DISCORD_CDN_URL}/guilds/{value}/users/{}/avatars/{hash}.{extension}", self.user_id());

        ImageSource::url(url).map_err(Into::into)
    }
}

impl AsImageSourceWith<Id<GuildMarker>> for Member {
    type Error = anyhow::Error;

    fn as_image_source_with(&self, value: Id<GuildMarker>) -> Result<ImageSource, Self::Error> {
        let url = if let Some(ref hash) = self.avatar {
            let extension = if hash.is_animated() { "gif" } else { "png" };

            format!("{DISCORD_CDN_URL}/guilds/{value}/users/{}/avatars/{hash}.{extension}", self.user.id)
        } else if let Some(ref hash) = self.user.avatar {
            let extension = if hash.is_animated() { "gif" } else { "png" };

            format!("{DISCORD_CDN_URL}/avatars/{}/{hash}.{extension}", self.user.id)
        } else {
            bail!("missing avatar hash");
        };

        ImageSource::url(url).map_err(Into::into)
    }
}

impl AsImageSourceWith<Id<GuildMarker>> for PartialMember {
    type Error = anyhow::Error;

    fn as_image_source_with(&self, value: Id<GuildMarker>) -> Result<ImageSource, Self::Error> {
        let Some(user_id) = self.user.as_ref().map(|u| u.id) else { bail!("missing user identifier") };
        let url = if let Some(ref hash) = self.avatar {
            let extension = if hash.is_animated() { "gif" } else { "png" };

            format!("{DISCORD_CDN_URL}/guilds/{value}/users/{user_id}/avatars/{hash}.{extension}")
        } else if let Some(hash) = self.user.as_ref().and_then(|u| u.avatar.as_ref()) {
            let extension = if hash.is_animated() { "gif" } else { "png" };

            format!("{DISCORD_CDN_URL}/avatars/{user_id}/{hash}.{extension}")
        } else {
            bail!("missing avatar hash");
        };

        ImageSource::url(url).map_err(Into::into)
    }
}

/// Converts the implementing type into a locale.
pub trait AsLocale {
    /// The error that may be returned when converting.
    type Error;

    /// Fallibly converts this value into a translation locale.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_locale(&self) -> Result<Locale, Self::Error>;
}

impl AsLocale for CachedGuild {
    type Error = <Locale as FromStr>::Err;

    fn as_locale(&self) -> Result<Locale, Self::Error> {
        self.preferred_locale().parse()
    }
}

impl AsLocale for CurrentUser {
    type Error = ina_localizing::Error;

    fn as_locale(&self) -> Result<Locale, Self::Error> {
        self.locale.as_deref().ok_or(ina_localizing::Error::MissingLocale)?.parse().map_err(Into::into)
    }
}

impl AsLocale for Guild {
    type Error = <Locale as FromStr>::Err;

    fn as_locale(&self) -> Result<Locale, Self::Error> {
        self.preferred_locale.parse()
    }
}

impl AsLocale for Interaction {
    type Error = ina_localizing::Error;

    fn as_locale(&self) -> Result<Locale, Self::Error> {
        self.locale.as_deref().ok_or(ina_localizing::Error::MissingLocale)?.parse().map_err(Into::into)
    }
}

impl AsLocale for Member {
    type Error = ina_localizing::Error;

    fn as_locale(&self) -> Result<Locale, Self::Error> {
        self.user.as_locale()
    }
}

impl AsLocale for PartialMember {
    type Error = ina_localizing::Error;

    fn as_locale(&self) -> Result<Locale, Self::Error> {
        self.user.as_ref().ok_or(ina_localizing::Error::MissingLocale)?.as_locale()
    }
}

impl AsLocale for TemplateGuild {
    type Error = <Locale as FromStr>::Err;

    fn as_locale(&self) -> Result<Locale, Self::Error> {
        self.preferred_locale.parse()
    }
}

impl AsLocale for User {
    type Error = ina_localizing::Error;

    fn as_locale(&self) -> Result<Locale, Self::Error> {
        self.locale.as_deref().ok_or(ina_localizing::Error::MissingLocale)?.parse().map_err(Into::into)
    }
}
