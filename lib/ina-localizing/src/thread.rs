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
use std::sync::{Arc, RwLock};

use ina_threading::statics::Static;
use ina_threading::threads::callable::StatefulCallableJoinHandle;
use tokio::runtime::Handle;

use crate::locale::Locale;
use crate::settings::Settings;
use crate::text::Text;
use crate::{Localizer, Result};

/// The localization thread's handle.
static HANDLE: Static<JoinHandle> = Static::new();

/// The inner type of the thread's handle.
pub(crate) type JoinHandle = StatefulCallableJoinHandle<Request, Response, RwLock<Localizer>>;

/// A request sent to the localization thread.
#[derive(Clone, Debug)]
pub enum Request {
    /// Lists all loaded locales.
    List,
    /// Returns whether the localizer has the loaded locales.
    Has(Box<[Locale]>),
    /// Clears the loaded locales, or the given locales if [`Some`] is contained within this variant.
    Clear(Option<Box<[Locale]>>),
    /// Loads the given locales, or the configured directory if [`None`] is contained within this variant.
    Load(Handle, Option<Box<[Locale]>>),
    /// Translates the given categorized key.
    Get(Handle, Option<Locale>, Box<str>, Box<str>),
    /// Returns a list of valid keys in the specified category.
    Keys(Option<Locale>, Box<str>),
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
    /// Returns a list of keys.
    Keys(Box<[Arc<str>]>),
}

/// Starts the localization thread.
///
/// # Errors
///
/// This function will return an error if the thread fails to spawn.
pub async fn start(settings: Settings) -> Result<()> {
    let capacity = settings.queue_capacity;
    let state = Arc::new(RwLock::new(Localizer::new(settings)));
    let handle = StatefulCallableJoinHandle::spawn(capacity, state, self::run)?;

    HANDLE.initialize(handle).await.map_err(Into::into)
}

/// Closes the localization thread.
pub async fn close() {
    HANDLE.uninitialize().await;
}

/// Runs the thread's process.
fn run((state, value): (Arc<RwLock<Localizer>>, Request)) -> Response {
    #[inline]
    fn read(state: &Arc<RwLock<Localizer>>) -> impl Deref<Target = Localizer> + '_ {
        assert!(!state.is_poisoned(), "storage was poisoned, possibly leading to corrupted data");

        state.read().unwrap_or_else(|_| unreachable!("the lock is guaranteed to not be poisoned"))
    }

    #[inline]
    fn write(state: &Arc<RwLock<Localizer>>) -> impl DerefMut<Target = Localizer> + '_ {
        assert!(!state.is_poisoned(), "storage was poisoned, possibly leading to corrupted data");

        state.write().unwrap_or_else(|_| unreachable!("the lock is guaranteed to not be poisoned"))
    }

    match value {
        Request::Get(runtime_handle, locale, category, key) => {
            let state = read(&state);
            let locale = locale.unwrap_or_else(|| state.settings.default_locale);

            match state.get(locale, &category, &key) {
                Ok(text) => {
                    if text.is_missing() {
                        let Text::Missing(category, key) = &text else {
                            unreachable!("the text is guaranteed to be missing at this point");
                        };

                        // This is error is intentionally ignored because it's better to return the text regardless of
                        // whether this log fails.
                        _ = runtime_handle.block_on(ina_logging::error!("missing text for key '{category}::{key}'"));
                    }

                    Response::Text(text)
                }
                Err(error) => Response::Error(Box::new(error)),
            }
        }
        Request::Has(locales) => {
            let state = read(&state);

            Response::Has(locales.iter().all(|l| state.has_locale(l)))
        }
        Request::List => {
            let state = read(&state);

            Response::List(state.locales().collect())
        }
        Request::Clear(locales) => {
            let mut state = write(&state);

            state.clear_locales(locales);

            Response::Acknowledge
        }
        Request::Load(_, Some(locales)) => {
            let mut state = write(&state);

            match state.load_locales(locales) {
                Ok(count) => Response::Load(count),
                Err(error) => Response::Error(Box::new(error)),
            }
        }
        Request::Load(runtime_handle, None) => {
            let mut state = write(&state);

            match state.load_directory(&runtime_handle) {
                Ok(count) => Response::Load(count),
                Err(error) => Response::Error(Box::new(error)),
            }
        }
        Request::Keys(locale, category) => {
            let state = read(&state);
            let locale = locale.unwrap_or_else(|| state.default_locale());

            Response::Keys(state.keys(&locale, &category).map_or_else(Box::default, |v| v.cloned().collect()))
        }
    }
}

/// Creates a thread invocation function.
macro_rules! invoke {
    ($(
        $(#[$attribute:meta])*
        $name:ident$(($($input:ident: $type:ty),*))? {
            $($request:tt)*
        } -> $return:ty {
            $($response:tt)*
        };
    )*) => {$(
        $(#[$attribute])*
        pub async fn $name($($($input: $type),*)?) -> Result<$return> {
            let response = HANDLE.try_get_mut().await?.invoke($($request)*).await?;

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
    list {
        Request::List
    } -> Box<[Locale]> {
        Response::List(list) => Ok(list),
    };

    /// Returns whether the given locales are loaded.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    has(locales: impl Send + IntoIterator<Item = Locale>) {
        Request::Has(locales.into_iter().collect())
    } -> bool {
        Response::Has(has) => Ok(has),
    };

    /// Clears the given locales, or all locales if `None` is provided.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    clear(locales: Option<impl Send + IntoIterator<Item = Locale>>) {
        Request::Clear(locales.map(|i| i.into_iter().collect()))
    } -> () {
        Response::Acknowledge => Ok(()),
    };

    /// Loads the given locales, returning the number of locales loaded.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    load(locales: Option<impl Send + IntoIterator<Item = Locale>>) {
        Request::Load(::tokio::runtime::Handle::current(), locales.map(|i| i.into_iter().collect()))
    } -> usize {
        Response::Load(count) => Ok(count),
    };

    /// Returns the locale's text assigned to the given categorized key.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    get(locale: Option<Locale>, category: impl Send + AsRef<str>, key: impl Send + AsRef<str>) {
        Request::Get(::tokio::runtime::Handle::current(), locale, category.as_ref().into(), key.as_ref().into())
    } -> Text {
        Response::Text(text) => Ok(text),
    };

    /// Returns the locale's stored keys in the given category.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    keys(locale: Option<Locale>, category: impl Send + AsRef<str>) {
        Request::Keys(locale, category.as_ref().into())
    } -> Box<[Arc<str>]> {
        Response::Keys(keys) => Ok(keys),
    };
}

/// Returns the localized text assigned to the given key and category.
///
/// # Examples
///
/// ```no_run
/// # use ina_localizing::localize;
/// # use ina_localizing::locale::Locale;
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), ina_localizing::Error> {
/// let locale = "es-MX".parse::<Locale>()?;
///
/// // In the specified optional locale.
/// localize!((try in Some(locale)) "ui", "test-key").await?;
/// // In the specified locale.
/// localize!((in locale) "ui", "test-key").await?;
/// // In the default locale ('en-US' by default).
/// localize!("ui", "test-key").await?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! localize {
    ((try in $locale:expr) $category:expr, $key:expr) => {
        $crate::thread::get($locale, $category, $key)
    };
    ((in $locale:expr) $category:expr, $key:expr) => {
        $crate::thread::get(Some($locale), $category, $key)
    };
    ($category:expr, $key:expr) => {
        $crate::thread::get(None, $category, $key)
    };
}
