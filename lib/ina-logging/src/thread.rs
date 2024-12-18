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

use ina_threading::ConsumingThread;
use ina_threading::join::Join;
use ina_threading::statics::JoinStatic;
use ina_threading::threads::consumer::Consumer;
use tokio::sync::mpsc::Receiver;

use crate::endpoint::Endpoint;
use crate::entry::{Entry, Level};
use crate::settings::Settings;
use crate::{Logger, Result};

/// The logging thread's handle.
static THREAD: LoggingThread = LoggingThread::new();

/// The logging thread's type.
pub type LoggingThread = JoinStatic<Consumer<Request, Result<()>>, Result<()>>;

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
    Endpoint(Box<dyn Endpoint>),
}

/// Starts the logging thread.
///
/// # Panics
///
/// Panics if the thread has already been initialized.
///
/// # Errors
///
/// This function will return an error if the thread could not be initialized.
pub async fn start(settings: Settings) -> Result<()> {
    assert!(!THREAD.async_api().has().await, "the thread has already been initialized");

    let capacity = settings.queue_capacity.get();
    let handle = Consumer::spawn_with_runtime("logging", |r| self::run(settings, r), capacity)?;
    let handle = Join::with_handle_handler(handle, |handle| {
        #[expect(clippy::expect_used, reason = "If the thread fails to close, logs will be lost silently")]
        handle.as_sender().blocking_send(Request::Close).expect("failed to close logging thread");
    });

    THREAD.async_api().set(handle).await;

    Ok(())
}

/// Starts the logging thread.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the thread has already been initialized or if this is called in an asynchronous context.
///
/// # Errors
///
/// This function will return an error if the thread could not be initialized.
pub fn blocking_start(settings: Settings) -> Result<()> {
    assert!(!THREAD.sync_api().has(), "the thread has already been initialized");

    let capacity = settings.queue_capacity.get();
    let handle = Consumer::spawn_with_runtime("logging", |r| self::run(settings, r), capacity)?;
    let handle = Join::with_handle_handler(handle, |handle| {
        #[expect(clippy::expect_used, reason = "If the thread fails to close, logs will be lost silently")]
        handle.as_sender().blocking_send(Request::Close).expect("failed to close logging thread");
    });

    THREAD.sync_api().set(handle);

    Ok(())
}

/// Requests that the logging thread outputs to the given endpoint.
///
/// # Panics
///
/// Panics if the thread has not been initialized.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn endpoint(endpoint: impl Endpoint) -> Result<()> {
    THREAD.async_api().get().await.as_sender().send(Request::Endpoint(Box::new(endpoint))).await?;

    Ok(())
}

/// Requests that the logging thread outputs to the given endpoint.
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
pub fn blocking_endpoint(endpoint: impl Endpoint) -> Result<()> {
    THREAD.sync_api().get().as_sender().blocking_send(Request::Endpoint(Box::new(endpoint)))?;

    Ok(())
}

/// Requests that the logging thread outputs the given log.
///
/// # Panics
///
/// Panics if the thread has not been initialized.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn entry(level: Level<'static>, text: impl Display + Send) -> Result<()> {
    let entry = Entry::new(level, text.to_string().into_boxed_str());

    THREAD.async_api().get().await.as_sender().send(Request::Entry(entry)).await?;

    Ok(())
}

/// Requests that the logging thread outputs the given log.
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
pub fn blocking_entry(level: Level<'static>, text: impl Display) -> Result<()> {
    let entry = Entry::new(level, text.to_string().into_boxed_str());

    THREAD.sync_api().get().as_sender().blocking_send(Request::Entry(entry))?;

    Ok(())
}

/// Requests that the logging thread flushes its buffer.
///
/// # Panics
///
/// Panics if the thread has not been initialized.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn flush() -> Result<()> {
    THREAD.async_api().get().await.as_sender().send(Request::Flush).await?;

    Ok(())
}

/// Requests that the logging thread flushes its buffer.
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
pub fn blocking_flush() -> Result<()> {
    THREAD.sync_api().get().as_sender().blocking_send(Request::Flush)?;

    Ok(())
}

/// Initializes the logging thread.
///
/// # Panics
///
/// Panics if the logging thread is not initialized.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn setup() -> Result<()> {
    THREAD.async_api().get().await.as_sender().send(Request::Setup).await?;

    Ok(())
}

/// Initializes the logging thread.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the logging thread is not initialized or if this is called in an asynchronous context.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub fn blocking_setup() -> Result<()> {
    THREAD.sync_api().get().as_sender().blocking_send(Request::Setup)?;

    Ok(())
}

/// Returns whether the thread has been started.
pub async fn is_started() -> bool {
    THREAD.async_api().has().await
}

/// Returns whether the thread has been started.
///
/// # Panics
///
/// Panics if this is called in an asynchronous context.
pub fn blocking_is_started() -> bool {
    THREAD.sync_api().has()
}

/// Closes the logging thread.
///
/// # Panics
///
/// Panics if the logging thread is not initialized.
pub async fn close() {
    assert!(THREAD.async_api().has().await, "the thread is not initialized");

    THREAD.async_api().drop().await;
}

/// Closes the logging thread.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the logging thread is not initialized or if this is called in an asynchronous context.
pub fn blocking_close() {
    assert!(THREAD.sync_api().has(), "the thread is not initialized");

    THREAD.sync_api().drop();
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
                None | Some(Request::Close) => return logger.close().await,
            },
        }
    }
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
/// debug!(async "This is an asynchronous debug log!").await?;
///
/// debug!("This is a synchronous debug log!")?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! debug {
    (async $($args:tt)+) => {{
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
    ($($args:tt)+) => {{
        #[cfg(debug_assertions)]
        {
            $crate::thread::blocking_entry($crate::entry::Level::DEBUG, ::std::format_args!($($args)+))
        }
        #[cfg(not(debug_assertions))]
        {
            ::std::mem::drop(::std::format_args!($($args)+));

            $crate::Result::<()>::Ok(())
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
/// info!(async "This is an asynchronous information log!").await?;
///
/// info!("This is a synchronous information log!")?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! info {
    (async $($args:tt)+) => {{
        $crate::thread::entry($crate::entry::Level::INFO, ::std::format!($($args)+))
    }};
    ($($args:tt)+) => {{
        $crate::thread::blocking_entry($crate::entry::Level::INFO, ::std::format_args!($($args)+))
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
/// warn!(async "This is an asynchronous warning log!").await?;
///
/// warn!("This is a synchronous warning log!")?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! warn {
    (async $($args:tt)+) => {{
        $crate::thread::entry($crate::entry::Level::WARN, ::std::format!($($args)+))
    }};
    ($($args:tt)+) => {{
        $crate::thread::blocking_entry($crate::entry::Level::WARN, ::std::format_args!($($args)+))
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
/// error!(async "This is an asynchronous error log!").await?;
///
/// error!("This is a synchronous error log!")?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! error {
    (async $($args:tt)+) => {{
        $crate::thread::entry($crate::entry::Level::ERROR, ::std::format!($($args)+))
    }};
    ($($args:tt)+) => {{
        $crate::thread::blocking_entry($crate::entry::Level::ERROR, ::std::format_args!($($args)+))
    }};
}
