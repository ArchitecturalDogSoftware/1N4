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
use std::future::Future;

use anyhow::Result;
use ina_localization::{localize, Locale, OwnedTranslation};
use time::macros::datetime;
use time::{Duration, OffsetDateTime};
use twilight_cache_inmemory::model::CachedGuild;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::guild::{Guild, Member, PartialMember};
use twilight_model::id::Id;
use twilight_model::user::{CurrentUser, User};
use twilight_util::builder::embed::EmbedAuthorBuilder;

/// Fallibly converts a reference to the implementing type into an embed author.
pub trait AsEmbedAuthor: Sized {
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
    type Error = Infallible;

    fn as_embed_author(&self) -> Result<EmbedAuthor, Self::Error> {
        Ok(EmbedAuthorBuilder::new(self.name()).build())
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
    fn blocking_localized(&self, locale: Option<Locale>) -> Result<OwnedTranslation<ina_localization::thread::Inner>> {
        localize!((try in locale) self.category(), self.key()).map_err(Into::into)
    }
}

impl<T: Localizable + Send + Sync> Localizable for &T {
    #[inline]
    fn category(&self) -> &str {
        <T as Localizable>::category(self)
    }

    #[inline]
    fn key(&self) -> &str {
        <T as Localizable>::key(self)
    }

    #[inline]
    fn localized(
        &self,
        locale: Option<Locale>,
    ) -> impl Future<Output = Result<OwnedTranslation<ina_localization::thread::Inner>>> + Send
    where
        Self: Send + Sync,
    {
        <T as Localizable>::localized(self, locale)
    }

    #[inline]
    fn blocking_localized(&self, locale: Option<Locale>) -> Result<OwnedTranslation<ina_localization::thread::Inner>> {
        <T as Localizable>::blocking_localized(self, locale)
    }
}

/// A value that has a preferred locale.
pub trait LocaleHolder {
    /// Returns this value's preferred locale, if set.
    fn preferred_locale(&self) -> Option<Locale> {
        None
    }
}

impl<T: LocaleHolder> LocaleHolder for &T {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> {
        <T as LocaleHolder>::preferred_locale(self)
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
