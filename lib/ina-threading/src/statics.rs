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

use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};

use crate::Handle;
use crate::joining::Joining;

/// A thread handle that can be stored within a static variable.
#[derive(Debug, Default)]
pub struct Static<H> {
    /// The inner thread handle.
    inner: RwLock<Option<H>>,
}

impl<H> Static<H> {
    /// Creates a new [`Static<H>`].
    pub const fn new() -> Self {
        Self { inner: RwLock::const_new(None) }
    }

    /// Creates a new [`Static<H>`] containing the given handle.
    pub const fn wrap(handle: H) -> Self {
        Self { inner: RwLock::const_new(Some(handle)) }
    }

    /// Returns an asynchronous API for this [`Static<H>`].
    pub const fn async_api(&self) -> AsyncStaticApi<H>
    where
        H: Send + Sync,
    {
        AsyncStaticApi { inner: &self.inner }
    }

    /// Returns a synchronous API for this [`Static<H>`].
    pub const fn sync_api(&self) -> SyncStaticApi<H> {
        SyncStaticApi { inner: &self.inner }
    }
}

/// A joining thread handle that can be stored within a static variable.
#[derive(Default)]
pub struct StaticJoining<H>
where
    H: Handle,
{
    /// The inner thread handle.
    inner: RwLock<Option<Joining<H>>>,
}

impl<H> StaticJoining<H>
where
    H: Handle,
{
    /// Creates a new [`StaticJoining<H>`].
    pub const fn new() -> Self {
        Self { inner: RwLock::const_new(None) }
    }

    /// Creates a new [`StaticJoining<H>`] containing the given handle.
    pub const fn wrap(handle: Joining<H>) -> Self {
        Self { inner: RwLock::const_new(Some(handle)) }
    }

    /// Returns an asynchronous API for this [`StaticJoining<H>`].
    pub const fn async_api(&self) -> AsyncStaticApi<Joining<H>>
    where
        H: Send + Sync,
    {
        AsyncStaticApi { inner: &self.inner }
    }

    /// Returns a synchronous API for this [`StaticJoining<H>`].
    pub const fn sync_api(&self) -> SyncStaticApi<Joining<H>> {
        SyncStaticApi { inner: &self.inner }
    }
}

/// An API for a static thread that provides its functionality through asynchronous calls.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct AsyncStaticApi<'sth, H>
where
    H: Send + Sync,
{
    /// The inner lock.
    inner: &'sth RwLock<Option<H>>,
}

#[expect(clippy::expect_used, reason = "we panic to ensure that the thread has been initialized prior to access")]
impl<H> AsyncStaticApi<'_, H>
where
    H: Send + Sync,
{
    /// Returns whether the inner thread handle has been initialized.
    pub async fn is_initialized(&self) -> bool {
        self.inner.read().await.is_some()
    }

    /// Initializes the thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread is already initialized.
    pub async fn initialize(&self, handle: H) {
        assert!(!self.is_initialized().await, "the thread is already initialized");

        *self.inner.write().await = Some(handle);
    }

    /// Closes the thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized.
    pub async fn close(&self) {
        self.inner.write().await.take().expect("the thread has not been initialized");
    }

    /// Returns a reference to the inner thread handle.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized.
    pub async fn get(&self) -> RwLockReadGuard<H> {
        RwLockReadGuard::map(self.inner.read().await, |v| v.as_ref().expect("the thread has not been initialized"))
    }

    /// Returns a mutable reference to the inner thread handle.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized.
    pub async fn get_mut(&self) -> RwLockMappedWriteGuard<H> {
        RwLockWriteGuard::map(self.inner.write().await, |v| v.as_mut().expect("the thread has not been initialized"))
    }

    /// Returns the inner thread handle, if it has been initialized.
    pub async fn take(&self) -> Option<H> {
        self.inner.write().await.take()
    }
}

/// An API for a static thread that provides its functionality through synchronous calls.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct SyncStaticApi<'sth, H> {
    /// The inner lock.
    inner: &'sth RwLock<Option<H>>,
}

#[expect(clippy::expect_used, reason = "we panic to ensure that the thread has been initialized prior to access")]
impl<H> SyncStaticApi<'_, H> {
    /// Returns whether the inner thread handle has been initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.inner.blocking_read().is_some()
    }

    /// Initializes the thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread is already initialized or this is called from within an asynchronous context.
    pub fn initialize(&self, handle: H) {
        assert!(!self.is_initialized(), "the thread is already initialized");

        *self.inner.blocking_write() = Some(handle);
    }

    /// Closes the thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized.
    pub fn close(&self) {
        self.inner.blocking_write().take().expect("the thread has not been initialized");
    }

    /// Returns a reference to the inner thread handle.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized or this is called from within an asynchronous context.
    pub fn get(&self) -> RwLockReadGuard<H> {
        RwLockReadGuard::map(self.inner.blocking_read(), |v| v.as_ref().expect("the thread has not been initialized"))
    }

    /// Returns a mutable reference to the inner thread handle.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized or this is called from within an asynchronous context.
    #[must_use]
    pub fn get_mut(&self) -> RwLockMappedWriteGuard<H> {
        RwLockWriteGuard::map(self.inner.blocking_write(), |v| v.as_mut().expect("the thread has not been initialized"))
    }

    /// Returns the inner thread handle, if it has been initialized.
    ///
    /// # Panics
    ///
    /// Panics if this is called from within an asynchronous context.
    #[must_use]
    pub fn take(&self) -> Option<H> {
        self.inner.blocking_write().take()
    }
}
