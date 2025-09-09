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
use std::sync::Arc;

use ina_threading::join::Join;
use ina_threading::statics::Static;
use ina_threading::threads::consumer::ConsumerJoinHandle;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::sync::RwLock;
use tokio::sync::mpsc::Receiver;

use crate::endpoint::Endpoint;
use crate::entry::{Entry, Level};
use crate::settings::Settings;
use crate::{Logger, Result};

/// The logging thread's handle.
static HANDLE: Static<JoinHandle> = Static::new();

/// The inner type of the thread's handle.
pub(crate) type JoinHandle = Join<ConsumerJoinHandle<Request, Result<()>>>;

/// A request send to the logging thread.
#[derive(Debug)]
pub enum Request {
    /// Initializes the logger.
    Setup,
    /// Closes the logging thread.
    Close,
    /// Flush the queue of the thread's logger.
    Flush,
    /// Queues an entry to be output during the next flush.
    Entry(Entry<'static>),
    /// Adds an endpoint to the logger.
    Endpoint(Arc<RwLock<dyn Endpoint>>),
}

/// Starts the logging thread.
///
/// # Errors
///
/// This function will return an error if the thread could not be initialized.
#[expect(clippy::missing_panics_doc, reason = "this function does not cause the panic")]
pub async fn start(settings: Settings) -> Result<()> {
    let handle = Join::new(ConsumerJoinHandle::spawn(settings.queue_capacity, |receiver| {
        RuntimeBuilder::new_current_thread().enable_all().build()?.block_on(self::run(settings, receiver))
    })?)
    .first(|handle| {
        #[expect(clippy::expect_used, reason = "If the thread fails to close, logs will be lost silently")]
        handle.sender().blocking_send(Request::Close).expect("failed to close logging thread");
    });

    HANDLE.initialize(handle).await?;

    Ok(())
}

/// Closes the logging thread.
pub async fn close() {
    HANDLE.uninitialize().await;
}

/// Runs the thread process.
///
/// # Errors
///
/// This function will return an error if the thread fails to run.
async fn run(settings: Settings, mut receiver: Receiver<Request>) -> Result<()> {
    let mut logger = Logger::new(settings);
    let mut duration = tokio::time::interval(logger.duration());

    loop {
        tokio::select! {
            _ = duration.tick() => logger.flush().await?,
            request = receiver.recv() => match request {
                Some(Request::Entry(entry)) => logger.push_entry(entry).await?,
                Some(Request::Flush) => logger.flush().await?,
                Some(Request::Endpoint(endpoint)) => logger.push_endpoint(endpoint).await?,
                Some(Request::Setup) => logger.setup().await?,
                Some(Request::Close) | None => return logger.close().await,
            },
        }
    }
}

/// Requests that the logging thread outputs to the given endpoint.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn endpoint(endpoint: impl Endpoint) -> Result<()> {
    let request = Request::Endpoint(Arc::new(RwLock::new(endpoint)));

    HANDLE.try_get().await?.sender().send(request).await.map_err(Into::into)
}

/// Requests that the logging thread outputs the given log.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn entry(level: Level<'static>, text: impl Display + Send) -> Result<()> {
    let request = Request::Entry(Entry::new(level, text.to_string().into()));

    HANDLE.try_get().await?.sender().send(request).await.map_err(Into::into)
}

/// Requests that the logging thread flushes its buffer.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn flush() -> Result<()> {
    HANDLE.try_get().await?.sender().send(Request::Flush).await.map_err(Into::into)
}

/// Initializes the logging thread.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn setup() -> Result<()> {
    HANDLE.try_get().await?.sender().send(Request::Setup).await.map_err(Into::into)
}

/// Returns whether the thread has been started.
pub async fn is_started() -> bool {
    HANDLE.is_initialized().await
}

/// Outputs a debug log.
///
/// # Examples
///
/// ```no_run
/// # use ina_logging::debug;
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), ina_logging::Error> {
/// debug!("This is an asynchronous debug log!").await?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! debug {
    ($($args:tt)+) => {{
        #[cfg(debug_assertions)]
        {
            $crate::thread::entry($crate::entry::Level::DEBUG, ::std::format!($($args)+))
        }
        #[cfg(not(debug_assertions))]
        {
            ::std::mem::drop(::std::format_args!($($args)+));

            ::std::future::ready($crate::Result::<()>::Ok(()))
        }
    }};
}

/// Outputs an information log.
///
/// # Examples
///
/// ```no_run
/// # use ina_logging::info;
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), ina_logging::Error> {
/// info!("This is an asynchronous information log!").await?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! info {
    ($($args:tt)+) => {{
        $crate::thread::entry($crate::entry::Level::INFO, ::std::format!($($args)+))
    }};
}

/// Outputs a warning log.
///
/// # Examples
///
/// ```no_run
/// # use ina_logging::warn;
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), ina_logging::Error> {
/// warn!("This is an asynchronous warning log!").await?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! warn {
    ($($args:tt)+) => {{
        $crate::thread::entry($crate::entry::Level::WARN, ::std::format!($($args)+))
    }};
}

/// Outputs an error log.
///
/// # Examples
///
/// ```no_run
/// # use ina_logging::error;
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), ina_logging::Error> {
/// error!("This is an asynchronous error log!").await?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! error {
    ($($args:tt)+) => {{
        $crate::thread::entry($crate::entry::Level::ERROR, ::std::format!($($args)+))
    }};
}
