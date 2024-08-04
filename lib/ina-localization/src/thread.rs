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
use std::ops::Deref;
use std::sync::Arc;

use ina_threading::{StatefulInvoker, Static};
use tokio::sync::RwLock;

use crate::{Error, Locale, Localizer, OwnedTranslation, Result, Settings};

/// The localization thread handle.
static THREAD: LocalizationThread = LocalizationThread::new();

/// The localization thread's type.
pub type LocalizationThread<T = Inner> = Static<StatefulInvoker<Localizer, Request<T>, Response<T>>, ()>;
/// The type stored within returned translations.
pub type Inner = Box<str>;

/// A request sent to the localization thread.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Request<T = Inner>
where
    T: Deref<Target = str>,
{
    /// Clears the loaded locales.
    Clear,
    /// Lists all loaded locales.
    List,
    /// Localizes the given key.
    Localize(Option<Locale>, TranslationKey<T>),
    /// Loads the given locale.
    Load(Option<Locale>),
}

/// A response sent from the localization thread.
#[derive(Debug)]
pub enum Response<T = Inner>
where
    T: Deref<Target = str>,
{
    /// The locales were cleared.
    Clear,
    /// A list of locales.
    List(Box<[Locale]>),
    /// The localized text.
    #[allow(clippy::type_complexity)]
    Localize(Result<OwnedTranslation<T>, (Option<usize>, (Arc<RwLock<Localizer>>, Request))>),
    /// The number of loaded locales.
    #[allow(clippy::type_complexity)]
    Load(Result<usize, (Option<usize>, (Arc<RwLock<Localizer>>, Request))>),
}

/// A translation key.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TranslationKey<T = Inner>(T, T)
where
    T: Deref<Target = str>;

impl<T> TranslationKey<T>
where
    T: Deref<Target = str>,
{
    /// Creates a new [`TranslationKey<T>`].
    #[inline]
    pub const fn new(category: T, key: T) -> Self {
        Self(category, key)
    }

    /// Returns a reference to the category of this [`TranslationKey<T>`].
    #[inline]
    pub fn category(&self) -> &str {
        &self.0
    }

    /// Returns a reference to the key of this [`TranslationKey<T>`].
    #[inline]
    pub fn key(&self) -> &str {
        &self.1
    }
}

impl<T, S> From<(S, S)> for TranslationKey<T>
where
    T: Deref<Target = str> + From<S>,
{
    #[inline]
    fn from((category, key): (S, S)) -> Self {
        Self(category.into(), key.into())
    }
}

/// Starts the localization thread.
///
/// # Panics
///
/// Panics if the thread has already been initialized.
///
/// # Errors
///
/// This function will return an error if the thread fails to spawn.
#[allow(clippy::type_complexity)]
pub async fn start(settings: Settings) -> Result<(), (Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    assert!(!THREAD.async_api().has().await);

    let capacity = settings.queue_capacity.get();
    let localizer = Localizer::new(settings);

    THREAD.async_api().set(StatefulInvoker::spawn_with_runtime("localization", localizer, self::run, capacity)?).await;

    Ok(())
}

/// Starts the localization thread.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the thread has already been initialized or if this is called in an asynchronous context.
///
/// # Errors
///
/// This function will return an error if the thread fails to spawn.
#[allow(clippy::type_complexity)]
pub fn blocking_start(settings: Settings) -> Result<(), (Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    assert!(!THREAD.sync_api().has());

    let capacity = settings.queue_capacity.get();
    let localizer = Localizer::new(settings);

    THREAD.sync_api().set(StatefulInvoker::spawn_with_runtime("localization", localizer, self::run, capacity)?);

    Ok(())
}

/// Closes the localization thread.
///
/// # Panics
///
/// Panics if the localization thread is not initialized.
pub async fn close() {
    assert!(THREAD.async_api().has().await, "the thread is not initialized");

    THREAD.async_api().drop().await;
}

/// Closes the localization thread.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the localization thread is not initialized or if this is called in an asynchronous context.
pub fn blocking_close() {
    assert!(THREAD.sync_api().has(), "the thread is not initialized");

    THREAD.sync_api().drop();
}

/// Clears all loaded locales.
///
/// # Panics
///
/// Panics if the thread has not been initialized.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
#[allow(clippy::panic, clippy::type_complexity)]
pub async fn clear() -> Result<(), (Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    let response = THREAD.async_api().get_mut().await.invoke(Request::Clear).await?;
    let Response::Clear = response else { panic!("unexpected response") };

    Ok(())
}

/// Clears all loaded locales.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the thread has not been initialized or this is called in an asynchronous context.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
#[allow(clippy::panic, clippy::type_complexity)]
pub fn blocking_clear() -> Result<(), (Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    let response = THREAD.sync_api().get_mut().blocking_invoke(Request::Clear)?;
    let Response::Clear = response else { panic!("unexpected response") };

    Ok(())
}

/// Lists all loaded locales.
///
/// # Panics
///
/// Panics if the thread has not been initialized.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
#[allow(clippy::panic, clippy::type_complexity)]
pub async fn list() -> Result<Box<[Locale]>, (Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    let response = THREAD.async_api().get_mut().await.invoke(Request::List).await?;
    let Response::List(locales) = response else { panic!("unexpected response") };

    Ok(locales)
}

/// Lists all loaded locales.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the thread has not been initialized or this is called in an asynchronous context.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
#[allow(clippy::panic, clippy::type_complexity)]
pub fn blocking_list() -> Result<Box<[Locale]>, (Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    let response = THREAD.sync_api().get_mut().blocking_invoke(Request::List)?;
    let Response::List(locales) = response else { panic!("unexpected response") };

    Ok(locales)
}

/// Localizes the given translation key.
///
/// # Panics
///
/// Panics if the thread has not been initialized.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
#[allow(clippy::panic, clippy::type_complexity)]
pub async fn localize(
    locale: Option<Locale>,
    key: TranslationKey,
) -> Result<OwnedTranslation<Inner>, (Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    let response = THREAD.async_api().get_mut().await.invoke(Request::Localize(locale, key)).await?;
    let Response::Localize(translation) = response else { panic!("unexpected response") };

    translation
}

/// Localizes the given translation key.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the thread has not been initialized or this is called in an asynchronous context.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
#[allow(clippy::panic, clippy::type_complexity)]
pub fn blocking_localize(
    locale: Option<Locale>,
    key: TranslationKey,
) -> Result<OwnedTranslation<Inner>, (Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    let response = THREAD.sync_api().get_mut().blocking_invoke(Request::Localize(locale, key))?;
    let Response::Localize(translation) = response else { panic!("unexpected response") };

    translation
}

/// Loads the given locale, or the configured directory if `None` is provided.
///
/// # Panics
///
/// Panics if the thread has not been initialized.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
#[allow(clippy::panic, clippy::type_complexity)]
pub async fn load(locale: Option<Locale>) -> Result<usize, (Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    let response = THREAD.async_api().get_mut().await.invoke(Request::Load(locale)).await?;
    let Response::Load(count) = response else { panic!("unexpected response") };

    count
}

/// Loads the given locale, or the configured directory if `None` is provided.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the thread has not been initialized or this is called in an asynchronous context.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
#[allow(clippy::panic, clippy::type_complexity)]
pub fn blocking_load(locale: Option<Locale>) -> Result<usize, (Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    let response = THREAD.sync_api().get_mut().blocking_invoke(Request::Load(locale))?;
    let Response::Load(count) = response else { panic!("unexpected response") };

    count
}

/// Runs the thread's process.
///
/// # Errors
///
/// This function will return an error if the thread fails to localize the key.
#[allow(clippy::needless_pass_by_value)]
async fn run<T>(localizer: Arc<RwLock<Localizer>>, input: Request<T>) -> Response<T>
where
    T: Deref<Target = str> + Send + for<'s> From<&'s str>,
{
    match input {
        Request::Clear => {
            localizer.write().await.clear_locales();

            Response::Clear
        }
        Request::List => Response::List(localizer.read().await.locales().collect()),
        Request::Localize(locale, key) => {
            let localizer = localizer.read().await;
            let locale = locale.unwrap_or_else(|| localizer.settings.default_locale);
            let result = localizer.get(locale, key.category(), key.key()).map(|v| v.as_owned());

            drop(localizer);

            Response::Localize(result.map_err(Into::into))
        }
        Request::Load(Some(locale)) => {
            let result = localizer.write().await.load_locale(locale).await;

            Response::Load(result.map(|()| 1).map_err(Into::into))
        }
        Request::Load(None) => {
            let result = localizer.write().await.load_directory().await;

            Response::Load(result.map_err(Into::into))
        }
    }
}

impl From<Error<Infallible>> for Error<(Option<usize>, (Arc<RwLock<Localizer>>, Request))> {
    fn from(value: Error<Infallible>) -> Self {
        match value {
            Error::FromToml(error) => Self::FromToml(error),
            Error::InvalidCharacter(character) => Self::InvalidCharacter(character),
            Error::InvalidLocale(locale) => Self::InvalidLocale(locale),
            Error::Io(error) => Self::Io(error),
            Error::MissingCharacter => Self::MissingCharacter,
            Error::MissingLocale => Self::MissingLocale,
            Error::MissingTranslation => Self::MissingTranslation,
            Error::Send(_) => unreachable!(),
            Error::Threading(error) => Self::Threading(match error {
                ina_threading::Error::Io(error) => ina_threading::Error::Io(error),
                ina_threading::Error::Disconnected => ina_threading::Error::Disconnected,
                ina_threading::Error::Send(_) => unreachable!(),
                _ => unimplemented!(),
            }),
        }
    }
}

/// Returns the localized text assigned to the given key and category.
///
/// # Examples
///
/// ```
/// let locale = "es-MX".parse::<Locale>()?;
///
/// // In the specified optional locale.
/// localize!(async(try in Some(locale)) "ui", "test-key").await?;
/// // In the specified locale.
/// localize!(async(in locale) "ui", "test-key").await?;
/// // In the default locale ('en-US' by default).
/// localize!(async "ui", "test-key").await?;
///
/// // In the specified optional locale.
/// localize!((try in Some(locale)) "ui", "test-key")?;
/// // In the specified locale.
/// localize!((in locale) "ui", "test-key")?;
/// // In the default locale ('en-US' by default).
/// localize!("ui", "test-key")?;
/// ```
#[macro_export]
macro_rules! localize {
    (async(try in $locale:expr) $category:expr, $key:expr) => {
        $crate::thread::localize($locale, $crate::thread::TranslationKey::new($category.into(), $key.into()))
    };
    (async(in $locale:expr) $category:expr, $key:expr) => {
        $crate::thread::localize(Some($locale), $crate::thread::TranslationKey::new($category.into(), $key.into()))
    };
    (async $category:expr, $key:expr) => {
        $crate::thread::localize(None, $crate::thread::TranslationKey::new($category.into(), $key.into()))
    };
    ((try in $locale:expr) $category:expr, $key:expr) => {
        $crate::thread::blocking_localize($locale, $crate::thread::TranslationKey::new($category.into(), $key.into()))
    };
    ((in $locale:expr) $category:expr, $key:expr) => {
        $crate::thread::blocking_localize(
            Some($locale),
            $crate::thread::TranslationKey::new($category.into(), $key.into()),
        )
    };
    ($category:expr, $key:expr) => {
        $crate::thread::blocking_localize(None, $crate::thread::TranslationKey::new($category.into(), $key.into()))
    };
}
