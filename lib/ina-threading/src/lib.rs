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

//! Provides concurrency solutions for 1N4.

use std::convert::Infallible;
use std::thread::{Builder, JoinHandle};

use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};

/// Provides utilities for automatically joining threads.
pub mod join;
/// Provides utilities for static threads.
pub mod statics;
/// Provides definitions for custom threads.
pub mod threads {
    /// Defines threads that consume values.
    pub mod consumer;
    /// Defines threads that consume and produce values.
    pub mod exchanger;
    /// Defines threads that can be called like functions.
    pub mod invoker;
    /// Defines threads that produce values.
    pub mod producer;
}

/// A result alias with a defaulted error type.
pub type Result<T, S = Infallible> = std::result::Result<T, Error<S>>;

/// An error that may occur when using this library.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error<S = Infallible> {
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A send error.
    #[error(transparent)]
    Send(#[from] SendError<S>),
    /// The thread was disconnected while awaiting a response.
    #[error("the thread was disconnected whilst awaiting a response")]
    Disconnected,
}

/// A thread with an associated join handle.
pub trait HandleHolder<T>
where
    T: Send + 'static,
{
    /// Returns a reference to the inner thread handle.
    fn as_handle(&self) -> &JoinHandle<T>;

    /// Returns a mutable reference to the inner thread handle.
    fn as_handle_mut(&mut self) -> &mut JoinHandle<T>;

    /// Unwraps this structure into the inner thread handle.
    fn into_handle(self) -> JoinHandle<T>;
}

/// A thread that consumes values.
pub trait ConsumingThread<S>
where
    S: Send + 'static,
{
    /// Returns a reference to the inner sender channel.
    fn as_sender(&self) -> &Sender<S>;

    /// Returns a mutable reference to the inner sender channel.
    fn as_sender_mut(&mut self) -> &mut Sender<S>;

    /// Unwraps this structure into the inner sender channel.
    fn into_sender(self) -> Sender<S>;

    /// Returns a clone of the inner sender.
    fn clone_sender(&self) -> Sender<S>;
}

/// A thread that produces values.
pub trait ProducingThread<R>
where
    R: Send + 'static,
{
    /// Returns a reference to the inner receiver channel.
    fn as_receiver(&self) -> &Receiver<R>;

    /// Returns a mutable reference to the inner receiver channel.
    fn as_receiver_mut(&mut self) -> &mut Receiver<R>;

    /// Unwraps this structure into the inner receiver channel.
    fn into_receiver(self) -> Receiver<R>;
}

/// A thread.
#[repr(transparent)]
#[derive(Debug)]
pub struct Thread<T>
where
    T: Send + 'static,
{
    /// The inner join handle.
    inner: JoinHandle<T>,
}

impl<T> Thread<T>
where
    T: Send + 'static,
{
    /// Spawns a new thread with the given name and task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F, S>(name: N, call: F) -> Result<Self, S>
    where
        N: AsRef<str>,
        F: FnOnce() -> T + Send + 'static,
    {
        let name = name.as_ref().replace('\0', r"\0");
        let inner = Builder::new().name(name).spawn(call)?;

        Ok(Self { inner })
    }
}

impl<T> HandleHolder<T> for Thread<T>
where
    T: Send + 'static,
{
    fn as_handle(&self) -> &JoinHandle<T> {
        &self.inner
    }

    fn as_handle_mut(&mut self) -> &mut JoinHandle<T> {
        &mut self.inner
    }

    fn into_handle(self) -> JoinHandle<T> {
        self.inner
    }
}
