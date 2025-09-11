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

//! Allows join handles to be easily stored as static variables.

use std::sync::OnceLock;

use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};

/// An error that may be returned when interacting with static thread handles.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error<H> {
    /// The thread has been initialized.
    #[error("the thread handle has been initialized")]
    Initialized(H),
    /// The thread has not been initialized.
    #[error("the thread handle has not been initialized")]
    Uninitialized,
}

/// A thread handle that can be stored as a static variable.
#[derive(Debug, Default)]
pub struct Static<H> {
    /// The inner thread handle.
    handle: RwLock<OnceLock<H>>,
}

impl<H> Static<H> {
    /// Creates a new uninitialized static thread handle.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::JoinHandle;
    /// # use ina_threading::statics::Static;
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// assert!(HANDLE.is_uninitialized().await);
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self { handle: RwLock::const_new(OnceLock::new()) }
    }

    /// Returns `true` if the inner thread has been initialized.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::{JoinHandle, JoinHandleWrapper};
    /// # use ina_threading::statics::Static;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// HANDLE.initialize(JoinHandle::spawn(|| ())?).await.unwrap();
    ///
    /// assert!(HANDLE.is_initialized().await);
    /// # HANDLE.uninitialize().await.unwrap().into_join_handle().join().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub async fn is_initialized(&self) -> bool
    where
        H: Sync,
    {
        self.handle.read().await.get().is_some()
    }

    /// Returns `true` if the inner thread has been initialized.
    ///
    /// # Panics
    ///
    /// This function will panic if called from within an asynchronous context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::{JoinHandle, JoinHandleWrapper};
    /// # use ina_threading::statics::Static;
    /// #
    /// # fn main() -> std::io::Result<()> {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// HANDLE.blocking_initialize(JoinHandle::spawn(|| ())?).unwrap();
    ///
    /// assert!(HANDLE.blocking_is_initialized());
    /// # HANDLE.blocking_uninitialize().unwrap().into_join_handle().join().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn blocking_is_initialized(&self) -> bool
    where
        H: Sync,
    {
        self.handle.blocking_read().get().is_some()
    }

    /// Returns `true` if the inner thread is uninitialized.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::JoinHandle;
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// assert!(HANDLE.is_uninitialized().await);
    /// # }
    /// ```
    #[inline]
    pub async fn is_uninitialized(&self) -> bool
    where
        H: Sync,
    {
        self.handle.read().await.get().is_none()
    }

    /// Returns `true` if the inner thread is uninitialized.
    ///
    /// # Panics
    ///
    /// This function will panic if called from within an asynchronous context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::JoinHandle;
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # fn main() {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// assert!(HANDLE.blocking_is_uninitialized());
    /// # }
    /// ```
    #[inline]
    pub fn blocking_is_uninitialized(&self) -> bool
    where
        H: Sync,
    {
        self.handle.blocking_read().get().is_none()
    }

    /// Initializes the inner thread handle.
    ///
    /// # Errors
    ///
    /// This function will return the provided handle if the thread has already been initialized.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::{JoinHandle, JoinHandleWrapper};
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// HANDLE.initialize(JoinHandle::spawn(|| ())?).await.unwrap();
    ///
    /// assert!(HANDLE.is_initialized().await);
    /// # HANDLE.uninitialize().await.unwrap().into_join_handle().join().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub async fn initialize(&self, handle: H) -> Result<(), Error<H>>
    where
        H: Sync,
    {
        self.handle.write().await.set(handle).map_err(Error::Initialized)
    }

    /// Initializes the inner thread handle.
    ///
    /// # Errors
    ///
    /// This function will return the provided handle if the thread has already been initialized.
    ///
    /// # Panics
    ///
    /// This function will panic if called from within an asynchronous context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::{JoinHandle, JoinHandleWrapper};
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # fn main() -> std::io::Result<()> {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// HANDLE.blocking_initialize(JoinHandle::spawn(|| ())?).unwrap();
    ///
    /// assert!(HANDLE.blocking_is_initialized());
    /// # HANDLE.blocking_uninitialize().unwrap().into_join_handle().join().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn blocking_initialize(&self, handle: H) -> Result<(), Error<H>>
    where
        H: Sync,
    {
        self.handle.blocking_write().set(handle).map_err(Error::Initialized)
    }

    /// Uninitializes the inner thread handle, returning it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::{JoinHandle, JoinHandleWrapper};
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// HANDLE.initialize(JoinHandle::spawn(|| ())?).await.unwrap();
    ///
    /// assert!(HANDLE.is_initialized().await);
    ///
    /// HANDLE.uninitialize().await.unwrap().into_join_handle().join().unwrap();
    ///
    /// assert!(HANDLE.is_uninitialized().await);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub async fn uninitialize(&self) -> Option<H>
    where
        H: Sync,
    {
        self.handle.write().await.take()
    }

    /// Uninitializes the inner thread handle, returning it.
    ///
    /// # Panics
    ///
    /// This function will panic if called from within an asynchronous context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::{JoinHandle, JoinHandleWrapper};
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # fn main() -> std::io::Result<()> {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// HANDLE.blocking_initialize(JoinHandle::spawn(|| ())?).unwrap();
    ///
    /// assert!(HANDLE.blocking_is_initialized());
    ///
    /// HANDLE.blocking_uninitialize().unwrap().into_join_handle().join().unwrap();
    ///
    /// assert!(HANDLE.blocking_is_uninitialized());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn blocking_uninitialize(&self) -> Option<H>
    where
        H: Sync,
    {
        self.handle.blocking_write().take()
    }

    /// Returns a reference to the inner thread handle.
    ///
    /// # Errors
    ///
    /// This function will return an error if the inner thread handle has not been initialized.
    ///
    /// # Examples
    ///
    /// Calling `.try_get()` on an uninitialized handle will always return an error.
    ///
    /// ```
    /// # use ina_threading::JoinHandle;
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// assert!(HANDLE.try_get().await.is_err_and(|error| matches!(error, Error::Uninitialized)));
    /// # }
    /// ```
    ///
    /// Calling `.try_get()` on an initialized handle will give you a guard that dereferences into the inner handle
    /// type.
    ///
    /// ```
    /// # use std::num::NonZero;
    /// #
    /// # use ina_threading::JoinHandleWrapper;
    /// # use ina_threading::threads::consumer::ConsumerJoinHandle;
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// static HANDLE: Static<ConsumerJoinHandle<u8, u8>> = Static::new();
    ///
    /// let capacity = NonZero::new(1).unwrap();
    /// let handle = ConsumerJoinHandle::<u8, u8>::spawn(capacity, |mut receiver| {
    ///     receiver.blocking_recv().unwrap().wrapping_pow(2)
    /// })?;
    ///
    /// HANDLE.initialize(handle).await.unwrap();
    ///
    /// let read_guard = HANDLE.try_get().await.unwrap();
    ///
    /// read_guard.sender().send(8).await.unwrap();
    ///
    /// drop(read_guard);
    ///
    /// let result = HANDLE.uninitialize().await.unwrap().into_join_handle().join().unwrap();
    ///
    /// assert_eq!(result, 64);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub async fn try_get(&self) -> Result<RwLockReadGuard<'_, H>, Error<H>>
    where
        H: Sync,
    {
        let guard = self.handle.read().await;

        if guard.get().is_some() {
            // The `.wait` call will never block because the handle is guaranteed to be present.
            Ok(RwLockReadGuard::map(guard, |lock| lock.wait()))
        } else {
            Err(Error::Uninitialized)
        }
    }

    /// Returns a reference to the inner thread handle.
    ///
    /// # Errors
    ///
    /// This function will return an error if the inner thread handle has not been initialized.
    ///
    /// # Panics
    ///
    /// This function will panic if called from within an asynchronous context.
    ///
    /// # Examples
    ///
    /// Calling `.blocking_try_get()` on an uninitialized handle will always return an error.
    ///
    /// ```
    /// # use ina_threading::JoinHandle;
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # fn main() {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// assert!(HANDLE.blocking_try_get().is_err_and(|error| matches!(error, Error::Uninitialized)));
    /// # }
    /// ```
    ///
    /// Calling `.blocking_try_get()` on an initialized handle will give you a guard that dereferences into the inner
    /// handle type.
    ///
    /// ```
    /// # use std::num::NonZero;
    /// #
    /// # use ina_threading::JoinHandleWrapper;
    /// # use ina_threading::threads::consumer::ConsumerJoinHandle;
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # fn main() -> std::io::Result<()> {
    /// static HANDLE: Static<ConsumerJoinHandle<u8, u8>> = Static::new();
    ///
    /// let capacity = NonZero::new(1).unwrap();
    /// let handle = ConsumerJoinHandle::<u8, u8>::spawn(capacity, |mut receiver| {
    ///     receiver.blocking_recv().unwrap().wrapping_pow(2)
    /// })?;
    ///
    /// HANDLE.blocking_initialize(handle).unwrap();
    ///
    /// let read_guard = HANDLE.blocking_try_get().unwrap();
    ///
    /// read_guard.sender().blocking_send(8).unwrap();
    ///
    /// drop(read_guard);
    ///
    /// let result = HANDLE.blocking_uninitialize().unwrap().into_join_handle().join().unwrap();
    ///
    /// assert_eq!(result, 64);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn blocking_try_get(&self) -> Result<RwLockReadGuard<'_, H>, Error<H>>
    where
        H: Sync,
    {
        let guard = self.handle.blocking_read();

        if guard.get().is_some() {
            // The `.wait` call will never block because the handle is guaranteed to be present.
            Ok(RwLockReadGuard::map(guard, |lock| lock.wait()))
        } else {
            Err(Error::Uninitialized)
        }
    }

    /// Returns a reference to the inner thread handle.
    ///
    /// # Errors
    ///
    /// This function will return an error if the inner thread handle has not been initialized.
    ///
    /// # Examples
    ///
    /// Calling `.try_get_mut()` on an uninitialized handle will always return an error.
    ///
    /// ```
    /// # use ina_threading::JoinHandle;
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// assert!(HANDLE.try_get_mut().await.is_err_and(|error| matches!(error, Error::Uninitialized)));
    /// # }
    /// ```
    ///
    /// Calling `.try_get_mut()` on an initialized handle will give you a guard that dereferences into the inner handle
    /// type.
    ///
    /// ```
    /// # use std::num::NonZero;
    /// #
    /// # use ina_threading::JoinHandleWrapper;
    /// # use ina_threading::threads::consumer::ConsumerJoinHandle;
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// static HANDLE: Static<ConsumerJoinHandle<u8, u8>> = Static::new();
    ///
    /// let capacity = NonZero::new(1).unwrap();
    /// let handle = ConsumerJoinHandle::<u8, u8>::spawn(capacity, |mut receiver| {
    ///     receiver.blocking_recv().unwrap().wrapping_pow(2)
    /// })?;
    ///
    /// HANDLE.initialize(handle).await.unwrap();
    ///
    /// let write_guard = HANDLE.try_get_mut().await.unwrap();
    ///
    /// write_guard.sender().send(8).await.unwrap();
    ///
    /// drop(write_guard);
    ///
    /// let result = HANDLE.uninitialize().await.unwrap().into_join_handle().join().unwrap();
    ///
    /// assert_eq!(result, 64);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub async fn try_get_mut(&self) -> Result<RwLockMappedWriteGuard<'_, H>, Error<H>>
    where
        H: Sync,
    {
        let guard = self.handle.write().await;

        if guard.get().is_some() {
            Ok(RwLockWriteGuard::map(guard, |lock: &mut OnceLock<H>| {
                lock.get_mut().unwrap_or_else(|| {
                    unreachable!("the lock is guaranteed to be initialized at this point");
                })
            }))
        } else {
            Err(Error::Uninitialized)
        }
    }

    /// Returns a reference to the inner thread handle.
    ///
    /// # Errors
    ///
    /// This function will return an error if the inner thread handle has not been initialized.
    ///
    /// # Panics
    ///
    /// This function will panic if called from within an asynchronous context.
    ///
    /// # Examples
    ///
    /// Calling `.blocking_try_get_mut()` on an uninitialized handle will always return an error.
    ///
    /// ```
    /// # use ina_threading::JoinHandle;
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # fn main() {
    /// static HANDLE: Static<JoinHandle<()>> = Static::new();
    ///
    /// assert!(
    ///     HANDLE.blocking_try_get_mut().is_err_and(|error| matches!(error, Error::Uninitialized))
    /// );
    /// # }
    /// ```
    ///
    /// Calling `.blocking_try_get_mut()` on an initialized handle will give you a guard that dereferences into the
    /// inner handle type.
    ///
    /// ```
    /// # use std::num::NonZero;
    /// #
    /// # use ina_threading::JoinHandleWrapper;
    /// # use ina_threading::threads::consumer::ConsumerJoinHandle;
    /// # use ina_threading::statics::{Error, Static};
    /// #
    /// # fn main() -> std::io::Result<()> {
    /// static HANDLE: Static<ConsumerJoinHandle<u8, u8>> = Static::new();
    ///
    /// let capacity = NonZero::new(1).unwrap();
    /// let handle = ConsumerJoinHandle::<u8, u8>::spawn(capacity, |mut receiver| {
    ///     receiver.blocking_recv().unwrap().wrapping_pow(2)
    /// })?;
    ///
    /// HANDLE.blocking_initialize(handle).unwrap();
    ///
    /// let write_guard = HANDLE.blocking_try_get_mut().unwrap();
    ///
    /// write_guard.sender().blocking_send(8).unwrap();
    ///
    /// drop(write_guard);
    ///
    /// let result = HANDLE.blocking_uninitialize().unwrap().into_join_handle().join().unwrap();
    ///
    /// assert_eq!(result, 64);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn blocking_try_get_mut(&self) -> Result<RwLockMappedWriteGuard<'_, H>, Error<H>>
    where
        H: Sync,
    {
        let guard = self.handle.blocking_write();

        if guard.get().is_some() {
            Ok(RwLockWriteGuard::map(guard, |lock: &mut OnceLock<H>| {
                lock.get_mut().unwrap_or_else(|| {
                    unreachable!("the lock is guaranteed to be initialized at this point");
                })
            }))
        } else {
            Err(Error::Uninitialized)
        }
    }
}
