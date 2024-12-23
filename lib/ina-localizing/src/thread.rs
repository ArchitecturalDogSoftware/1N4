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

use ina_threading::statics::Static;
use ina_threading::threads::invoker::{Stateful, StatefulInvoker};
use tokio::sync::RwLock;

use crate::locale::Locale;
use crate::settings::Settings;
use crate::text::Text;
use crate::{Localizer, Result};

/// The localization thread's handle.
static THREAD: LocalizationThread = LocalizationThread::new();

/// The localization thread's type.
pub type LocalizationThread = Static<StatefulInvoker<RwLock<Localizer>, Request, Response>>;

/// A request sent to the localization thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Lists all loaded locales.
    List,
    /// Returns whether the localizer has the loaded locales.
    Has(Box<[Locale]>),
    /// Clears the loaded locales, or the given locales if [`Some`] is contained within this variant.
    Clear(Option<Box<[Locale]>>),
    /// Loads the given locales, or the configured directory if [`None`] is contained within this variant.
    Load(Option<Box<[Locale]>>),
    /// Translates the given categorized key.
    Get(Option<Locale>, Box<str>, Box<str>),
}

/// A response sent from the localization thread.
#[derive(Debug)]
pub enum Response {
    /// Acknowledges a request.
    Acknowledge,
    /// Fails a request.
    Error(Box<crate::Error>),
    /// Lists locales.
    List(Box<[Locale]>),
    /// Returns whether the locales were loaded.
    Has(bool),
    /// Returns the result of loading locales.
    Load(usize),
    /// Returns translated text.
    Text(Text),
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
pub async fn start(settings: Settings) -> Result<()> {
    let capacity = settings.queue_capacity;
    let localizer = RwLock::new(Localizer::new(settings));
    let handle = StatefulInvoker::spawn_with_runtime("localizing", capacity, localizer, self::run)?;

    THREAD.async_api().initialize(handle).await;

    Ok(())
}

/// Starts the localization thread, blocking the current thread until successful.
///
/// # Panics
///
/// Panics if the thread has already been initialized or if this is called from within an asynchronous context.
///
/// # Errors
///
/// This function will return an error if the thread fails to spawn.
pub fn blocking_start(settings: Settings) -> Result<()> {
    assert!(!THREAD.sync_api().is_initialized(), "the thread has already been initialized");

    let capacity = settings.queue_capacity;
    let localizer = RwLock::new(Localizer::new(settings));
    let handle = StatefulInvoker::spawn_with_runtime("localizing", capacity, localizer, self::run)?;

    THREAD.sync_api().initialize(handle);

    Ok(())
}

/// Closes the localization thread.
///
/// # Panics
///
/// Panics if the localization thread is not initialized.
pub async fn close() {
    THREAD.async_api().close().await;
}

/// Closes the localization thread.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the localization thread is not initialized or if this is called in an asynchronous context.
pub fn blocking_close() {
    THREAD.sync_api().close();
}

/// Runs the thread's process.
async fn run(Stateful { state, value }: Stateful<RwLock<Localizer>, Request>) -> Response {
    match value {
        Request::Get(locale, category, key) => {
            let state = state.read().await;
            let locale = locale.unwrap_or_else(|| state.settings.default_locale);

            match state.get(locale, &category, &key) {
                Ok(text) => {
                    if text.is_missing() && ina_logging::thread::is_started().await {
                        let Text::Missing(category, key) = &text else { unreachable!() };

                        // TODO: is it fine to ignore the possible error here?
                        // This is with the assumption that the logger will not fail to output.
                        ina_logging::error!(async "missing text for key '{category}::{key}'").await.ok();
                    }

                    Response::Text(text)
                }
                Err(error) => Response::Error(Box::new(error)),
            }
        }
        Request::Has(locales) => {
            let state = state.read().await;

            Response::Has(locales.iter().all(|l| state.has_locale(l)))
        }
        Request::List => {
            let state = state.read().await;

            Response::List(state.locales().collect())
        }
        Request::Clear(locales) => {
            let mut state = state.write().await;

            state.clear_locales(locales);

            Response::Acknowledge
        }
        Request::Load(Some(locales)) => {
            let mut state = state.write().await;

            match state.load_locales(locales).await {
                Ok(count) => Response::Load(count),
                Err(error) => Response::Error(Box::new(error)),
            }
        }
        Request::Load(None) => {
            let mut state = state.write().await;

            match state.load_directory().await {
                Ok(count) => Response::Load(count),
                Err(error) => Response::Error(Box::new(error)),
            }
        }
    }
}

/// Creates a thread invoker function.
macro_rules! invoke {
    ($(
        $(#[$attribute:meta])*
        $name:ident, $blocking_name:ident $(($($input:ident: $type:ty),*))? {
            $($request:tt)*
        } -> $return:ty {
            $($response:tt)*
        };
    )*) => {$(
        $(#[$attribute])*
        pub async fn $name($($($input: $type),*)?) -> Result<$return> {
            let response = THREAD.async_api().get_mut().await.call($($request)*).await?;

            match response {
                $($response)*
                Response::Error(error) => Err(*error),
                _ => unreachable!("unexpected response: '{response:?}'"),
            }
        }

        $(#[$attribute])*
        ///
        /// # Panics
        ///
        /// Panics if this is called from within a synchronous context.
        pub fn $blocking_name($($($input: $type),*)?) -> Result<$return> {
            let response = THREAD.sync_api().get_mut().blocking_call($($request)*)?;

            match response {
                $($response)*
                Response::Error(error) => Err(*error),
                _ => unreachable!("unexpected response: '{response:?}'"),
            }
        }
    )*};
}

invoke! {
    /// Lists all loaded locales.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    list, blocking_list {
        Request::List
    } -> Box<[Locale]> {
        Response::List(list) => Ok(list),
    };

    /// Returns whether the given locales are loaded.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    has, blocking_has (locales: impl Send + IntoIterator<Item = Locale>) {
        Request::Has(locales.into_iter().collect())
    } -> bool {
        Response::Has(has) => Ok(has),
    };

    /// Clears the given locales, or all locales if `None` is provided.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    clear, blocking_clear (locales: Option<impl Send + IntoIterator<Item = Locale>>) {
        Request::Clear(locales.map(|i| i.into_iter().collect()))
    } -> () {
        Response::Acknowledge => Ok(()),
    };

    /// Loads the given locales, returning the number of locales loaded.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    load, blocking_load (locales: Option<impl Send + IntoIterator<Item = Locale>>) {
        Request::Load(locales.map(|i| i.into_iter().collect()))
    } -> usize {
        Response::Load(count) => Ok(count),
    };

    /// Returns the locale's text assigned to the given categorized key.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    get, blocking_get (locale: Option<Locale>, category: impl Send + AsRef<str>, key: impl Send + AsRef<str>) {
        Request::Get(locale, category.as_ref().into(), key.as_ref().into())
    } -> Text {
        Response::Text(text) => Ok(text),
    };
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
        $crate::thread::get($locale, $category, $key)
    };
    (async(in $locale:expr) $category:expr, $key:expr) => {
        $crate::thread::get(Some($locale), $category, $key)
    };
    (async $category:expr, $key:expr) => {
        $crate::thread::get(None, $category, $key)
    };
    ((try in $locale:expr) $category:expr, $key:expr) => {
        $crate::thread::blocking_get($locale, $category, $key)
    };
    ((in $locale:expr) $category:expr, $key:expr) => {
        $crate::thread::blocking_get(Some($locale), $category, $key)
    };
    ($category:expr, $key:expr) => {
        $crate::thread::blocking_get(None, $category, $key)
    };
}
