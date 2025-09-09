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

//! Defines supplier threads.

use std::num::NonZero;
use std::ops::{Deref, DerefMut};

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{JoinHandle, JoinHandleWrapper};

/// A thread that has a linked channel through which data can be received.
#[derive(Debug)]
pub struct SupplierJoinHandle<R, T> {
    /// The receiver-end of the linked channel.
    receiver: Receiver<R>,
    /// The inner join handle.
    handle: JoinHandle<T>,
}

impl<R, T> SupplierJoinHandle<R, T> {
    /// Creates a new [`SupplierJoinHandle<S, T>`] using the given function.
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
    /// # use ina_threading::threads::supplier::SupplierJoinHandle;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let capacity = NonZero::new(2).unwrap();
    /// let mut handle = SupplierJoinHandle::<i32, ()>::spawn(capacity, |sender| {
    ///     sender.blocking_send(2).unwrap();
    ///     sender.blocking_send(5).unwrap();
    /// })?;
    ///
    /// let lhs = handle.receiver().recv().await.unwrap();
    /// let rhs = handle.receiver().recv().await.unwrap();
    ///
    /// assert_eq!(lhs + rhs, 7);
    /// # handle.into_join_handle().join().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn spawn<F>(capacity: NonZero<usize>, f: F) -> std::io::Result<Self>
    where
        R: Send + 'static,
        T: Send + 'static,
        F: FnOnce(Sender<R>) -> T + Send + 'static,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity.get());

        JoinHandle::spawn(|| f(sender)).map(|handle| Self { receiver, handle })
    }

    /// Returns a reference to the receiver of the linked channel.
    #[inline]
    #[must_use]
    pub const fn receiver(&mut self) -> &mut Receiver<R> {
        &mut self.receiver
    }
}

impl<R, T> JoinHandleWrapper for SupplierJoinHandle<R, T> {
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

impl<R, T> AsRef<std::thread::JoinHandle<T>> for SupplierJoinHandle<R, T> {
    #[inline]
    fn as_ref(&self) -> &std::thread::JoinHandle<T> {
        self.as_join_handle()
    }
}

impl<R, T> Deref for SupplierJoinHandle<R, T> {
    type Target = std::thread::JoinHandle<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_join_handle()
    }
}

impl<R, T> AsMut<std::thread::JoinHandle<T>> for SupplierJoinHandle<R, T> {
    #[inline]
    fn as_mut(&mut self) -> &mut std::thread::JoinHandle<T> {
        self.as_join_handle_mut()
    }
}

impl<R, T> DerefMut for SupplierJoinHandle<R, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_join_handle_mut()
    }
}

impl<R, T> From<SupplierJoinHandle<R, T>> for std::thread::JoinHandle<T> {
    #[inline]
    fn from(value: SupplierJoinHandle<R, T>) -> Self {
        value.into_join_handle()
    }
}
