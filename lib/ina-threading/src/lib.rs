// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024 Jaxydog
//
// This file is part of 1N4.
//
// 1N4 is free software: you can redistribute it and/or modify it under the terms of the GNU Affero
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// 1N4 is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the
// implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero
// General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License along with 1N4. If not,
// see <https://www.gnu.org/licenses/>.

//! Provides concurrency solutions for 1N4.

use std::convert::Infallible;
use std::ops::{Deref, DerefMut};
use std::thread::{Builder, JoinHandle};

use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};

pub use crate::threads::*;

/// Provides definitions for custom threads.
mod threads;

/// A result alias with a defaulted error type.
pub type Result<T, S = Infallible> = std::result::Result<T, Error<S>>;

/// An error that may occur when using this library.
#[derive(Debug, thiserror::Error)]
pub enum Error<S> {
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
    #[inline]
    fn as_handle(&self) -> &JoinHandle<T> {
        &self.inner
    }

    #[inline]
    fn as_handle_mut(&mut self) -> &mut JoinHandle<T> {
        &mut self.inner
    }

    #[inline]
    fn into_handle(self) -> JoinHandle<T> {
        self.inner
    }
}

/// A thread that is automatically joined when dropped.
pub struct Join<H, T>
where
    H: HandleHolder<T>,
    T: Send + 'static,
{
    /// The inner thread handle.
    inner: Option<H>,
    /// A function that is called before the thread is joined.
    clean_up_handle: Option<fn(&mut H)>,
    /// A function that is called after the thread is joined.
    clean_up_result: Option<fn(T)>,
}

impl<H, T> Join<H, T>
where
    H: HandleHolder<T>,
    T: Send + 'static,
{
    /// Creates a new [`Join<H, T>`] thread.
    pub(crate) const fn new(inner: H, clean_up_handle: Option<fn(&mut H)>, clean_up_result: Option<fn(T)>) -> Self {
        Self { inner: Some(inner), clean_up_handle, clean_up_result }
    }

    /// Thinly wraps the given handle.
    pub const fn wrap(inner: H) -> Self {
        Self::new(inner, None, None)
    }

    /// Wraps the given handle and runs the given function before the thread is joined.
    pub const fn clean_up_handle(inner: H, f: fn(&mut H)) -> Self {
        Self::new(inner, Some(f), None)
    }

    /// Wraps the given handle and runs the given function after the thread is joined.
    pub const fn clean_up_result(inner: H, f: fn(T)) -> Self {
        Self::new(inner, None, Some(f))
    }

    /// Wraps the given handle and runs the given function after the thread is joined.
    pub const fn clean_up_all(inner: H, handle: fn(&mut H), result: fn(T)) -> Self {
        Self::new(inner, Some(handle), Some(result))
    }
}

impl<H, T> Deref for Join<H, T>
where
    H: HandleHolder<T>,
    T: Send + 'static,
{
    type Target = H;

    #[allow(clippy::expect_used)]
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().expect("the thread has already been joined")
    }
}

impl<H, T> DerefMut for Join<H, T>
where
    H: HandleHolder<T>,
    T: Send + 'static,
{
    #[allow(clippy::expect_used)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().expect("the thread has already been joined")
    }
}

impl<H, T> Drop for Join<H, T>
where
    H: HandleHolder<T>,
    T: Send + 'static,
{
    #[allow(clippy::unwrap_used)]
    fn drop(&mut self) {
        let Some(mut thread) = self.inner.take() else { return };

        if let Some(clean_up_handle) = self.clean_up_handle {
            clean_up_handle(&mut thread);
        }

        let result = thread.into_handle().join().unwrap();

        self.clean_up_result.unwrap_or(drop)(result);
    }
}

/// Wraps a thread and allows it to be safely stored within a static.
pub enum Static<H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    /// Wraps a thread.
    Wrap(RwLock<Option<H>>),
    /// Wraps a joining thread.
    Join(RwLock<Option<Join<H, T>>>),
}

impl<H, T> Static<H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    /// Creates a new [`Static::Wrap`] variant.
    pub const fn wrap() -> Self {
        Self::Wrap(RwLock::const_new(None))
    }

    /// Creates a new [`Static::Join`] variant.
    pub const fn join() -> Self {
        Self::Join(RwLock::const_new(None))
    }

    /// Returns a reference to the inner thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized.
    pub async fn get(&self) -> StaticReadLock<H, T> {
        #[inline]
        fn extract<T>(lock: RwLockReadGuard<Option<T>>) -> RwLockReadGuard<T> {
            #[allow(clippy::expect_used)]
            RwLockReadGuard::map(lock, |v| v.as_ref().expect("the thread has not been initialized"))
        }

        match self {
            Self::Wrap(h) => StaticReadLock::Wrap(extract(h.read().await)),
            Self::Join(h) => StaticReadLock::Join(extract(h.read().await)),
        }
    }

    /// Returns a mutable reference to the inner thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized.
    pub async fn get_mut(&mut self) -> StaticWriteLock<H, T> {
        #[inline]
        fn extract<T>(lock: RwLockWriteGuard<Option<T>>) -> RwLockMappedWriteGuard<T> {
            #[allow(clippy::expect_used)]
            RwLockWriteGuard::map(lock, |v| v.as_mut().expect("the thread has not been initialized"))
        }

        match self {
            Self::Wrap(h) => StaticWriteLock::Wrap(extract(h.write().await)),
            Self::Join(h) => StaticWriteLock::Join(extract(h.write().await)),
        }
    }

    /// Returns a reference to the inner thread.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized or this is called in an asynchronous context.
    pub fn blocking_get(&self) -> StaticReadLock<H, T> {
        #[inline]
        fn extract<T>(lock: RwLockReadGuard<Option<T>>) -> RwLockReadGuard<T> {
            #[allow(clippy::expect_used)]
            RwLockReadGuard::map(lock, |v| v.as_ref().expect("the thread has not been initialized"))
        }

        match self {
            Self::Wrap(h) => StaticReadLock::Wrap(extract(h.blocking_read())),
            Self::Join(h) => StaticReadLock::Join(extract(h.blocking_read())),
        }
    }

    /// Returns a mutable reference to the inner thread.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if the thread has not been initialized or this is called in an asynchronous context.
    pub fn blocking_get_mut(&mut self) -> StaticWriteLock<H, T> {
        #[inline]
        fn extract<T>(lock: RwLockWriteGuard<Option<T>>) -> RwLockMappedWriteGuard<T> {
            #[allow(clippy::expect_used)]
            RwLockWriteGuard::map(lock, |v| v.as_mut().expect("the thread has not been initialized"))
        }

        match self {
            Self::Wrap(h) => StaticWriteLock::Wrap(extract(h.blocking_write())),
            Self::Join(h) => StaticWriteLock::Join(extract(h.blocking_write())),
        }
    }

    /// Drops the inner thread handle.
    ///
    /// This does nothing if the thread was never initialized.
    pub async fn drop(&self) {
        match self {
            Self::Wrap(h) => *h.write().await = None,
            Self::Join(h) => *h.write().await = None,
        }
    }

    /// Drops the inner thread handle.
    ///
    /// This does nothing if the thread was never initialized.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    pub fn blocking_drop(&self) {
        match self {
            Self::Wrap(h) => *h.blocking_write() = None,
            Self::Join(h) => *h.blocking_write() = None,
        }
    }
}

/// Wraps a lock for a static thread.
pub enum StaticReadLock<'rl, H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    /// Wraps a thread.
    Wrap(RwLockReadGuard<'rl, H>),
    /// Wraps a joining thread.
    Join(RwLockReadGuard<'rl, Join<H, T>>),
}

impl<'rl, H, T> Deref for StaticReadLock<'rl, H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    type Target = H;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Wrap(l) => l,
            Self::Join(l) => l,
        }
    }
}

/// Wraps a lock for a static thread.
pub enum StaticWriteLock<'wl, H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    /// Wraps a thread.
    Wrap(RwLockMappedWriteGuard<'wl, H>),
    /// Wraps a joining thread.
    Join(RwLockMappedWriteGuard<'wl, Join<H, T>>),
}

impl<'rl, H, T> Deref for StaticWriteLock<'rl, H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    type Target = H;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Wrap(l) => l,
            Self::Join(l) => l,
        }
    }
}

impl<'rl, H, T> DerefMut for StaticWriteLock<'rl, H, T>
where
    H: Send + Sync + HandleHolder<T>,
    T: Send + Sync + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Wrap(l) => l,
            Self::Join(l) => l,
        }
    }
}
