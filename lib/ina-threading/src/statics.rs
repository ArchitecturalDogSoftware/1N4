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

use std::marker::PhantomData;

use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};

use crate::join::Join;
use crate::HandleHolder;

/// A static thread handle.
#[derive(Debug)]
pub struct Static<H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    /// The inner handle, locked for mutability purposes.
    inner: RwLock<Option<H>>,
    /// The type marker for `T`.
    _marker: PhantomData<fn(H) -> T>,
}

impl<H, T> Static<H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    /// Creates a new [`Static<H, T>`].
    pub const fn new() -> Self {
        Self { inner: RwLock::const_new(None), _marker: PhantomData }
    }

    /// Creates a new [`Static<H, T>`] containing the given handle.
    pub const fn with_handle(handle: H) -> Self {
        Self { inner: RwLock::const_new(Some(handle)), _marker: PhantomData }
    }

    /// Returns the asynchronous API for this [`Static<H, T>`].
    pub const fn async_api(&self) -> AsyncStaticApi<H> {
        AsyncStaticApi { inner: &self.inner }
    }

    /// Returns the synchronous API for this [`Static<H, T>`].
    pub const fn sync_api(&self) -> SyncStaticApi<H> {
        SyncStaticApi { inner: &self.inner }
    }
}

impl<H, T> Default for Static<H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A joining static thread handle.
#[derive(Debug)]
pub struct JoinStatic<H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    /// The inner handle, locked for mutability purposes.
    inner: RwLock<Option<Join<H, T>>>,
}

impl<H, T> JoinStatic<H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    /// Creates a new [`JoinStatic<H, T>`].
    pub const fn new() -> Self {
        Self { inner: RwLock::const_new(None) }
    }

    /// Creates a new [`JoinStatic<H, T>`] containing the given handle.
    pub const fn with_handle(handle: Join<H, T>) -> Self {
        Self { inner: RwLock::const_new(Some(handle)) }
    }

    /// Returns the asynchronous API for this [`JoinStatic<H, T>`].
    pub const fn async_api(&self) -> AsyncStaticApi<Join<H, T>> {
        AsyncStaticApi { inner: &self.inner }
    }

    /// Returns the synchronous API for this [`JoinStatic<H, T>`].
    pub const fn sync_api(&self) -> SyncStaticApi<Join<H, T>> {
        SyncStaticApi { inner: &self.inner }
    }
}

impl<H, T> Default for JoinStatic<H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// An asynchronous API for a static thread.
#[repr(transparent)]
#[must_use = "api values do nothing unless used"]
#[derive(Clone, Copy, Debug)]
pub struct AsyncStaticApi<'s, T>
where
    T: Send + Sync,
{
    /// The inner locked handle.
    inner: &'s RwLock<Option<T>>,
}

#[allow(clippy::expect_used)]
impl<'s, T> AsyncStaticApi<'s, T>
where
    T: Send + Sync,
{
    /// Returns whether the inner thread is initialized.
    pub async fn has(&self) -> bool {
        self.inner.read().await.is_some()
    }

    /// Returns a reference to the inner thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized.
    pub async fn get(&self) -> RwLockReadGuard<T> {
        RwLockReadGuard::map(self.inner.read().await, |v| v.as_ref().expect("the thread is not initialized"))
    }

    /// Returns a mutable reference to the inner thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized.
    pub async fn get_mut(&self) -> RwLockMappedWriteGuard<T> {
        RwLockWriteGuard::map(self.inner.write().await, |v| v.as_mut().expect("the thread is not initialized"))
    }

    /// Sets the inner thread handle, dropping the old value.
    pub async fn set(&self, handle: T) {
        drop(self.inner.write().await.replace(handle));
    }

    /// Returns the inner thread handle.
    ///
    /// This returns [`None`] if the thread was never initialized.
    pub async fn take(&self) -> Option<T> {
        self.inner.write().await.take()
    }

    /// Drops the inner thread handle.
    ///
    /// This does nothing if the thread was never initialized.
    pub async fn drop(&self) {
        drop(self.inner.write().await.take());
    }
}

/// A synchronous API for a static thread.
#[repr(transparent)]
#[must_use = "api values do nothing unless used"]
#[derive(Clone, Copy, Debug)]
pub struct SyncStaticApi<'s, T> {
    inner: &'s RwLock<Option<T>>,
}

#[allow(clippy::expect_used)]
impl<'s, T> SyncStaticApi<'s, T> {
    /// Returns whether the inner thread is initialized.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized or this is called in an asynchronous context.
    #[must_use]
    pub fn has(&self) -> bool {
        self.inner.blocking_read().is_some()
    }

    /// Returns a reference to the inner thread.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized or this is called in an asynchronous context.
    pub fn get(&self) -> RwLockReadGuard<T> {
        RwLockReadGuard::map(self.inner.blocking_read(), |v| v.as_ref().expect("the thread is not initialized"))
    }

    /// Returns a mutable reference to the inner thread.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized or this is called in an asynchronous context.
    #[must_use]
    pub fn get_mut(&self) -> RwLockMappedWriteGuard<T> {
        RwLockWriteGuard::map(self.inner.blocking_write(), |v| v.as_mut().expect("the thread is not initialized"))
    }

    /// Sets the inner thread handle, dropping the old value.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    pub fn set(&self, handle: T) {
        drop(self.inner.blocking_write().replace(handle));
    }

    /// Returns the inner thread handle.
    ///
    /// This returns [`None`] if the thread was never initialized.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    #[must_use]
    pub fn take(&self) -> Option<T> {
        self.inner.blocking_write().take()
    }

    /// Drops the inner thread handle.
    ///
    /// This does nothing if the thread was never initialized.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    pub fn drop(&self) {
        drop(self.inner.blocking_write().take());
    }
}
