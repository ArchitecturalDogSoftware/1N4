// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2025 Jaxydog
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

//! Defines consumer threads.

use std::num::NonZero;
use std::ops::{Deref, DerefMut};

use tokio::runtime::Handle;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{JoinHandle, JoinHandleWrapper};

/// A thread that has a linked channel through which data can be sent.
#[derive(Debug)]
pub struct ConsumerJoinHandle<S, T> {
    /// The sender-end of the linked channel.
    sender: Sender<S>,
    /// The inner join handle.
    handle: JoinHandle<T>,
}

impl<S, T> ConsumerJoinHandle<S, T> {
    /// Creates a new [`ConsumerJoinHandle<S, T>`] using the given function.
    ///
    /// # Errors
    ///
    /// This function will return an error if the operating system fails to spawn the thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// #
    /// # use ina_threading::JoinHandleWrapper;
    /// # use ina_threading::threads::consumer::ConsumerJoinHandle;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let capacity = NonZero::new(2).unwrap();
    /// let handle = ConsumerJoinHandle::<i32, i32>::spawn(capacity, |mut receiver| {
    ///     let lhs = receiver.blocking_recv().unwrap();
    ///     let rhs = receiver.blocking_recv().unwrap();
    ///
    ///     lhs + rhs
    /// })?;
    ///
    /// handle.sender().send(2).await.unwrap();
    /// handle.sender().send(5).await.unwrap();
    ///
    /// assert_eq!(7, handle.into_join_handle().join().unwrap());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn spawn<F>(capacity: NonZero<usize>, f: F) -> std::io::Result<Self>
    where
        S: Send + 'static,
        T: Send + 'static,
        F: FnOnce(Receiver<S>) -> T + Send + 'static,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity.get());

        JoinHandle::spawn(|| f(receiver)).map(|handle| Self { sender, handle })
    }

    /// Creates a new [`ConsumerJoinHandle<S, T>`] using the given runtime handle and asynchronous function.
    ///
    /// # Errors
    ///
    /// This function will return an error if the operating system fails to spawn the thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// #
    /// # use ina_threading::JoinHandleWrapper;
    /// # use ina_threading::threads::consumer::ConsumerJoinHandle;
    /// # use tokio::runtime::Handle;
    /// # use tokio::sync::mpsc::Receiver;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let capacity = NonZero::new(2).unwrap();
    /// let handle = ConsumerJoinHandle::spawn_async(
    ///     Handle::current(),
    ///     capacity,
    ///     |mut receiver: Receiver<i32>| async move {
    ///         let lhs = receiver.recv().await.unwrap();
    ///         let rhs = receiver.recv().await.unwrap();
    ///
    ///         lhs + rhs
    ///     },
    /// )?;
    ///
    /// handle.sender().send(2).await.unwrap();
    /// handle.sender().send(5).await.unwrap();
    ///
    /// assert_eq!(7, handle.into_join_handle().join().unwrap());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn spawn_async<F>(handle: Handle, capacity: NonZero<usize>, f: F) -> std::io::Result<Self>
    where
        S: Send + 'static,
        T: Send + 'static,
        F: AsyncFnOnce(Receiver<S>) -> T + Send + 'static,
    {
        Self::spawn(capacity, move |receiver| handle.block_on(f(receiver)))
    }

    /// Returns a reference to the sender of the linked channel.
    #[inline]
    #[must_use]
    pub const fn sender(&self) -> &Sender<S> {
        &self.sender
    }
}

impl<S, T> JoinHandleWrapper for ConsumerJoinHandle<S, T> {
    type Output = T;

    #[inline]
    fn as_join_handle(&self) -> &std::thread::JoinHandle<T> {
        self.handle.as_join_handle()
    }

    #[inline]
    fn as_join_handle_mut(&mut self) -> &mut std::thread::JoinHandle<T> {
        self.handle.as_join_handle_mut()
    }

    #[inline]
    fn into_join_handle(self) -> std::thread::JoinHandle<T> {
        self.handle.into_join_handle()
    }
}

impl<S, T> AsRef<std::thread::JoinHandle<T>> for ConsumerJoinHandle<S, T> {
    #[inline]
    fn as_ref(&self) -> &std::thread::JoinHandle<T> {
        self.as_join_handle()
    }
}

impl<S, T> Deref for ConsumerJoinHandle<S, T> {
    type Target = std::thread::JoinHandle<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_join_handle()
    }
}

impl<S, T> AsMut<std::thread::JoinHandle<T>> for ConsumerJoinHandle<S, T> {
    #[inline]
    fn as_mut(&mut self) -> &mut std::thread::JoinHandle<T> {
        self.as_join_handle_mut()
    }
}

impl<S, T> DerefMut for ConsumerJoinHandle<S, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_join_handle_mut()
    }
}

impl<S, T> From<ConsumerJoinHandle<S, T>> for std::thread::JoinHandle<T> {
    #[inline]
    fn from(value: ConsumerJoinHandle<S, T>) -> Self {
        value.into_join_handle()
    }
}
