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

use ina_threading::{Consumer, ConsumingThread, Join, JoinStatic};
use tokio::sync::mpsc::Receiver;

use crate::{Entry, Level, Logger, Result, Settings};

/// The logging thread handle.
static THREAD: LoggingThread<'static> = LoggingThread::new();

/// The logging thread's type.
pub type LoggingThread<'lv> = JoinStatic<Consumer<Request<'lv>, Result<()>>, Result<()>>;

/// A request sent to the logging thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request<'lv> {
    /// Flushes the inner queue of the [`Logger`].
    Flush,
    /// Queues a log to be output during the next flush.
    Queue(Entry<'lv>),
}

/// Starts the logging thread.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the thread has already been initialized.
///
/// # Errors
///
/// This function will return an error if the logging thread fails to start.
#[allow(clippy::missing_panics_doc)] // This doesn't actually panic during the function call.
pub async fn start(settings: Settings) -> Result<(), Request<'static>> {
    assert!(!THREAD.async_api().has().await, "the thread has already been initialized");

    let capacity = settings.queue_capacity.get();
    let handle = Join::clean_up_handle(Consumer::spawn("logging", |r| run(settings, r), capacity)?, |handle| {
        #[allow(clippy::expect_used)]
        handle.as_sender().blocking_send(Request::Flush).expect("failed to flush logging thread");
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
/// This function will return an error if the logging thread fails to start.
#[allow(clippy::missing_panics_doc)] // This doesn't actually panic during the function call.
pub fn blocking_start(settings: Settings) -> Result<(), Request<'static>> {
    assert!(!THREAD.sync_api().has(), "the thread has already been initialized");

    let capacity = settings.queue_capacity.get();
    let handle = Join::clean_up_handle(Consumer::spawn("logging", |r| run(settings, r), capacity)?, |handle| {
        #[allow(clippy::expect_used)]
        handle.as_sender().blocking_send(Request::Flush).expect("failed to flush logging thread");
    });

    THREAD.sync_api().set(handle);

    Ok(())
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

/// Requests that the logging thread outputs the given log.
///
/// # Panics
///
/// Panics if the thread has not been initialized.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn queue(level: Level<'static>, text: impl Display + Send) -> Result<(), Request<'static>> {
    let entry = Entry::new(level, text.to_string());

    THREAD.async_api().get().await.as_sender().send(Request::Queue(entry)).await?;

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
pub fn blocking_queue(level: Level<'static>, text: impl Display) -> Result<(), Request<'static>> {
    let entry = Entry::new(level, text.to_string());

    THREAD.sync_api().get().as_sender().blocking_send(Request::Queue(entry))?;

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
pub async fn flush() -> Result<(), Request<'static>> {
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
pub fn blocking_flush() -> Result<(), Request<'static>> {
    THREAD.sync_api().get().as_sender().blocking_send(Request::Flush)?;

    Ok(())
}

/// Runs the thread process.
///
/// # Errors
///
/// This function will return an error if the thread fails to run.
fn run(settings: Settings, mut receiver: Receiver<Request<'static>>) -> Result<()> {
    let mut logger = Logger::<'static>::new(settings)?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_time().build()?;

    runtime.block_on(async {
        let mut timeout = tokio::time::interval(logger.timeout());

        loop {
            tokio::select! {
                _ = timeout.tick() => logger.flush()?,
                result = receiver.recv() => match result {
                    Some(Request::Queue(entry)) => logger.queue(entry)?,
                    Some(Request::Flush) => logger.flush()?,
                    None => return Ok(()),
                }
            }
        }
    })
}

/// Outputs a debug log.
///
/// # Examples
///
/// ```
/// debug!(async "This is a debug log!").await?;
/// debug!("This is a debug log!")?;
/// ```
#[macro_export]
macro_rules! debug {
    (async $($args:tt)+) => {{
        #[cfg(debug_assertions)]
        {
            $crate::thread::queue($crate::Level::DEBUG, ::std::format!($($args)+))
        }
        #[cfg(not(debug_assertions))]
        {
            // Prevents unused variable warnings.
            ::std::mem::drop(::std::format_args!($($args)+));
            ::std::future::ready($crate::Result::<(), $crate::thread::Request<'static>>::Ok(()))
        }
    }};
    ($($args:tt)+) => {{
        #[cfg(debug_assertions)]
        {
            $crate::thread::blocking_queue($crate::Level::DEBUG, ::std::format!($($args)+))
        }
        #[cfg(not(debug_assertions))]
        {
            // Prevents unused variable warnings.
            ::std::mem::drop(::std::format_args!($($args)+));
            $crate::Result::<(), $crate::thread::Request<'static>>::Ok(())
        }
    }};
}

/// Outputs an informational log.
///
/// # Examples
///
/// ```
/// info!(async "This is an informational log!").await?;
/// info!("This is an informational log!")?;
/// ```
#[macro_export]
macro_rules! info {
    (async $($args:tt)+) => {
        $crate::thread::queue($crate::Level::INFO, ::std::format!($($args)+))
    };
    ($($args:tt)+) => {
        $crate::thread::blocking_queue($crate::Level::INFO, ::std::format!($($args)+))
    };
}

/// Outputs a warning log.
///
/// # Examples
///
/// ```
/// warn!(async "This is a warning log!").await?;
/// warn!("This is a warning log!")?;
/// ```
#[macro_export]
macro_rules! warn {
    (async $($args:tt)+) => {
        $crate::thread::queue($crate::Level::WARN, ::std::format!($($args)+))
    };
    ($($args:tt)+) => {
        $crate::thread::blocking_queue($crate::Level::WARN, ::std::format!($($args)+))
    };
}

/// Outputs an error log.
///
/// # Examples
///
/// ```
/// error!(async "This is an error log!").await?;
/// error!("This is an error log!")?;
/// ```
#[macro_export]
macro_rules! error {
    (async $($args:tt)+) => {
        $crate::thread::queue($crate::Level::ERROR, ::std::format!($($args)+))
    };
    ($($args:tt)+) => {
        $crate::thread::blocking_queue($crate::Level::ERROR, ::std::format!($($args)+))
    };
}
