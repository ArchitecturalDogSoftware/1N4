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

//! Provides concurrency solutions for 1N4.

use std::ops::{Deref, DerefMut};

use tokio::runtime::Handle;

pub mod join;
pub mod statics;

/// Defines default implementations for common threading use-cases.
pub mod threads {
    pub mod callable;
    pub mod consumer;
    pub mod exchanger;
    pub mod supplier;
}

/// Represents a type that wraps a thread join handle.
pub trait JoinHandleWrapper {
    /// The value being returned by the handle.
    type Output;

    /// Returns an immutable reference to the inner join handle.
    fn as_join_handle(&self) -> &std::thread::JoinHandle<Self::Output>;

    /// Returns a mutable reference to the inner join handle.
    fn as_join_handle_mut(&mut self) -> &mut std::thread::JoinHandle<Self::Output>;

    /// Unwraps this value into the inner join handle.
    fn into_join_handle(self) -> std::thread::JoinHandle<Self::Output>;
}

/// A wrapper around the standard library's thread join handle type.
#[repr(transparent)]
#[derive(Debug)]
pub struct JoinHandle<T>(std::thread::JoinHandle<T>);

impl<T> JoinHandle<T> {
    /// Creates a new [`JoinHandle<T>`] using the given function.
    ///
    /// # Errors
    ///
    /// This function will return an error if the operating system fails to spawn the thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::time::{Duration, Instant};
    /// #
    /// # use ina_threading::JoinHandle;
    /// #
    /// # fn main() -> std::io::Result<()> {
    /// let instant = Instant::now();
    ///
    /// let handle = JoinHandle::spawn(|| {
    ///     std::thread::sleep(Duration::from_millis(1_000));
    ///
    ///     Instant::now()
    /// })?;
    ///
    /// assert!(instant < std::thread::JoinHandle::from(handle).join().unwrap());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn spawn<F>(f: F) -> std::io::Result<Self>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        std::thread::Builder::new().spawn(f).map(Self)
    }

    /// Creates a new [`JoinHandle<T>`] using the given runtime handle and asynchronous function.
    ///
    /// # Errors
    ///
    /// This function will return an error if the operating system fails to spawn the thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::JoinHandle;
    /// # use tokio::runtime::Handle;
    /// # use tokio::time::{Duration, Instant};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let instant = Instant::now();
    ///
    /// let handle = JoinHandle::spawn_async(Handle::current(), || async {
    ///     tokio::time::sleep(Duration::from_millis(1_000)).await;
    ///
    ///     Instant::now()
    /// })?;
    ///
    /// assert!(instant < std::thread::JoinHandle::from(handle).join().unwrap());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn spawn_async<F>(handle: Handle, f: F) -> std::io::Result<Self>
    where
        T: Send + 'static,
        F: AsyncFnOnce() -> T + Send + 'static,
    {
        Self::spawn(move || handle.block_on(f()))
    }
}

impl<T> JoinHandleWrapper for JoinHandle<T> {
    type Output = T;

    #[inline]
    fn as_join_handle(&self) -> &std::thread::JoinHandle<T> {
        &self.0
    }

    #[inline]
    fn as_join_handle_mut(&mut self) -> &mut std::thread::JoinHandle<T> {
        &mut self.0
    }

    #[inline]
    fn into_join_handle(self) -> std::thread::JoinHandle<T> {
        self.0
    }
}

impl<T> AsRef<std::thread::JoinHandle<T>> for JoinHandle<T> {
    #[inline]
    fn as_ref(&self) -> &std::thread::JoinHandle<T> {
        self.as_join_handle()
    }
}

impl<T> Deref for JoinHandle<T> {
    type Target = std::thread::JoinHandle<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_join_handle()
    }
}

impl<T> AsMut<std::thread::JoinHandle<T>> for JoinHandle<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut std::thread::JoinHandle<T> {
        self.as_join_handle_mut()
    }
}

impl<T> DerefMut for JoinHandle<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_join_handle_mut()
    }
}

impl<T> From<JoinHandle<T>> for std::thread::JoinHandle<T> {
    #[inline]
    fn from(value: JoinHandle<T>) -> Self {
        value.into_join_handle()
    }
}
