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

//! Provides logging solutions for 1N4.

use std::sync::Arc;
use std::time::Duration;

use endpoint::Endpoint;
use settings::Settings;
use thread::Request;
use tokio::sync::RwLock;
use tokio::sync::mpsc::error::SendError;
use tokio::task::{JoinError, JoinSet};

use crate::entry::Entry;

/// Defines various output endpoints for the logger.
pub mod endpoint;
/// Defines log entries, the internal representation for logs.
pub mod entry;
/// Defines the logger's settings.
pub mod settings;
/// Defines the logger's thread.
pub mod thread;

/// A result that may be returned when using this library.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error that may be returned when using this library.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The logger was already initialized.
    #[error("the logger has already been initialized")]
    AlreadyInitialized,
    /// A duplicate endpoint.
    #[error("the '{0}' endpoint has already been added")]
    DuplicateEndpoint(&'static str),
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An invalid endpoint state.
    #[error("the '{0}' endpoint has an invalid state")]
    InvalidEndpointState(&'static str),
    /// The logger has not been initialized.
    #[error("the logger has not been initialized")]
    NotInitialized,
    /// A join error.
    #[error("a task failed to join")]
    Join(#[from] JoinError),
    /// A request failed to send.
    #[error("a request failed to send")]
    Send(#[from] SendError<Request>),
    /// A threading error.
    #[error(transparent)]
    Thread(#[from] ina_threading::Error<Request>),
}

/// A logger with buffered output.
pub struct Logger {
    /// The logger's settings.
    settings: Settings,
    /// The logger's endpoints.
    endpoints: Vec<Arc<RwLock<Box<dyn Endpoint>>>>,
    /// The logger's entry queue.
    queue: Vec<Arc<Entry<'static>>>,
    /// Whether the logger has been initialized.
    initialized: bool,
}

impl Logger {
    /// Creates a new [`Logger`].
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        let queue = Vec::with_capacity(settings.queue_capacity.get());

        Self { settings, endpoints: vec![], queue, initialized: false }
    }

    /// Returns whether this [`Logger`] is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        !self.settings.quiet && !self.endpoints.is_empty()
    }

    /// Returns whether this [`Logger`] is disabled.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.settings.quiet || self.endpoints.is_empty()
    }

    /// Returns the queue timeout of this [`Logger`].
    #[must_use]
    pub const fn duration(&self) -> Duration {
        Duration::from_millis(self.settings.queue_duration.get())
    }

    /// Returns the queue capacity of this [`Logger`].
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.settings.queue_capacity.get()
    }

    /// Returns whether this [`Logger`] has entry buffering.
    #[must_use]
    pub const fn is_buffered(&self) -> bool {
        self.capacity() > 1
    }

    /// Returns the number of entries within the inner queue of this [`Logger`].
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Returns whether the inner queue of this [`Logger`] is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns whether the inner queue of this [`Logger`] is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity()
    }

    /// Initializes the endpoints of this [`Logger`].
    ///
    /// # Errors
    ///
    /// This function will return an error if an endpoint fails to initialize, or if this has already been run.
    pub async fn setup(&mut self) -> Result<()> {
        if self.initialized {
            return Err(Error::AlreadyInitialized);
        }

        for endpoint in &self.endpoints {
            endpoint.write().await.setup(&self.settings).await?;
        }

        self.initialized = true;

        Ok(())
    }

    /// Adds an endpoint to the logger.
    ///
    /// # Errors
    ///
    /// This function will return an error if the endpoint was already added.
    pub async fn push_endpoint(&mut self, endpoint: Box<dyn Endpoint>) -> Result<()> {
        if self.initialized {
            return Err(Error::AlreadyInitialized);
        }

        for contained in &self.endpoints {
            if contained.read().await.name() == endpoint.name() {
                return Err(Error::DuplicateEndpoint(endpoint.name()));
            }
        }

        self.endpoints.push(Arc::new(RwLock::new(endpoint)));

        Ok(())
    }

    /// Adds an entry to the logger, flushing if its buffer capacity is met.
    ///
    /// # Errors
    ///
    /// This function will return an error if the logger could not be flushed.
    pub async fn push_entry(&mut self, entry: Entry<'static>) -> Result<()> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }
        if self.is_enabled() {
            self.queue.push(Arc::new(entry));
        }

        if self.is_full() { self.flush().await } else { Ok(()) }
    }

    /// Flushes the inner queue of this [`Logger`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the queue could not be flushed.
    pub async fn flush(&mut self) -> Result<()> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }
        if self.is_disabled() {
            self.queue.clear();

            return Ok(());
        }

        let mut tasks = JoinSet::<Result<(), Error>>::new();

        #[expect(clippy::unnecessary_to_owned, reason = "false positive")]
        for endpoint in self.endpoints.iter().cloned() {
            let iterator: Box<[_]> = self.queue.iter().cloned().collect();

            tasks.spawn(async move {
                let mut endpoint = endpoint.write().await;

                for entry in iterator {
                    endpoint.write(entry).await?;
                }

                drop(endpoint);

                Ok(())
            });
        }

        self.queue.clear();

        while let Some(result) = tasks.join_next().await {
            result??;
        }

        Ok(())
    }

    /// Closes the logger and its endpoints.
    ///
    /// # Errors
    ///
    /// This function will return an error if the logger fails to close.
    pub async fn close(mut self) -> Result<()> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        self.flush().await?;

        for endpoint in self.endpoints {
            endpoint.write().await.close().await?;
        }

        Ok(())
    }
}
