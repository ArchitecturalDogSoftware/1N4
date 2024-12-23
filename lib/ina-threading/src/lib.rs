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

use std::thread::{Builder, JoinHandle};

use tokio::sync::mpsc::{Receiver, Sender};

/// Defines wrappers for join-on-drop threads.
pub mod joining;
/// Defines wrappers for threads that are stored statically.
pub mod statics;

/// Defines specialized thread implementations.
pub mod threads {
    /// Defines threads that can receive values.
    pub mod consumer;
    /// Defines threads that can send and receive values.
    pub mod exchanger;
    /// Defines threads that can be called like functions.
    pub mod invoker;
    /// Defines threads that can send values.
    pub mod producer;
}

/// A result alias with a defaulted error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error that may occur when using this library.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// A type that contains a [`JoinHandle`] that represents a possibly running thread.
pub trait Handle {
    /// The type that is returned when the thread is closed.
    type Output: Send + 'static;

    /// Returns a reference to the contained [`JoinHandle`].
    fn as_join_handle(&self) -> &JoinHandle<Self::Output>;

    /// Returns a mutable reference to the contained [`JoinHandle`].
    fn as_join_handle_mut(&mut self) -> &mut JoinHandle<Self::Output>;

    /// Returns the contained [`JoinHandle`], dropping this value.
    fn into_join_handle(self) -> JoinHandle<Self::Output>;
}

/// A [`Handle`] type where the running thread may receive values through a [`Sender<T>`].
pub trait SenderHandle<T>
where
    Self: Handle,
    T: Send + 'static,
{
    /// Returns a reference to the contained [`Sender<T>`].
    fn as_sender(&self) -> &Sender<T>;

    /// Returns a mutable reference to the contained [`Sender<T>`].
    fn as_sender_mut(&mut self) -> &mut Sender<T>;

    /// Returns the contained [`Sender<T>`], dropping this value.
    fn into_sender(self) -> Sender<T>;
}

/// A [`Handle`] type where the running thread may send values through a [`Receiver<T>`].
pub trait ReceiverHandle<T>
where
    Self: Handle,
    T: Send + 'static,
{
    /// Returns a reference to the contained [`Receiver<T>`].
    fn as_receiver(&self) -> &Receiver<T>;

    /// Returns a mutable reference to the contained [`Receiver<T>`].
    fn as_receiver_mut(&mut self) -> &mut Receiver<T>;

    /// Returns the contained [`Receiver<T>`], dropping this value.
    fn into_receiver(self) -> Receiver<T>;
}

/// A simple thread with an associated handle.
#[repr(transparent)]
#[derive(Debug)]
pub struct Thread<T> {
    /// The inner [`JoinHandle<T>`].
    inner: JoinHandle<T>,
}

impl<T> Thread<T>
where
    T: Send + 'static,
{
    /// Spawns a new [`Thread`] with the given name and task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F>(name: N, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: FnOnce() -> T + Send + 'static,
    {
        let name = name.as_ref().replace('\0', r"\0");
        let inner = Builder::new().name(name).spawn(f)?;

        Ok(Self { inner })
    }

    /// Spawns a new [`Thread`] with the given name and asynchronous task.
    ///
    /// The created runtime has both IO and time drivers enabled, and is configured to only run on the spawned thread.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    #[expect(clippy::expect_used, reason = "if the runtime fails to spawn, we can't run the thread body")]
    #[expect(clippy::missing_panics_doc, reason = "the panic does not cause a crash, only stops the thread")]
    pub fn spawn_with_runtime<N, F, O>(name: N, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: FnOnce() -> O + Send + 'static,
        O: Future<Output = T>,
    {
        Self::spawn(name, || {
            use tokio::runtime::Builder;

            let runtime = Builder::new_current_thread().enable_all().build().expect("failed to spawn runtime");

            runtime.block_on(async move { f().await })
        })
    }
}

impl<T> Handle for Thread<T>
where
    T: Send + 'static,
{
    type Output = T;

    fn as_join_handle(&self) -> &JoinHandle<Self::Output> {
        &self.inner
    }

    fn as_join_handle_mut(&mut self) -> &mut JoinHandle<Self::Output> {
        &mut self.inner
    }

    fn into_join_handle(self) -> JoinHandle<Self::Output> {
        self.inner
    }
}
