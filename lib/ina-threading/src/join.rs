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

//! Implements a wrapper for join handles that automatically runs functions when they are dropped.

use std::any::Any;
use std::ops::{Deref, DerefMut};

use crate::JoinHandleWrapper;

/// A thread handle that automatically joins when it is dropped.
#[must_use = "this handle will automatically run its drop behavior and attempt to join immediately"]
#[derive(Debug)]
pub struct Join<H>
where
    H: JoinHandleWrapper,
{
    /// The inner thread handle.
    handle: Option<H>,
    /// The function applied to the handle before it is joined.
    first: Option<fn(&mut H)>,
    /// The function applied to the return value of the handle after it has joined.
    value: Option<fn(H::Output)>,
    /// The function called when the thread panics.
    panic: Option<fn(Box<dyn Any + Send + 'static>)>,
}

impl<H> Join<H>
where
    H: JoinHandleWrapper,
{
    /// The function used when no panic handler is specified.
    ///
    /// By default, this simply propagates the panic.
    #[expect(clippy::panic, reason = "if a thread panics, we assume that it was intentional and propagate it")]
    pub const DEFAULT_PANIC_FN: fn(Box<dyn Any + Send + 'static>) = |value| {
        std::panic::panic_any(value);
    };

    /// Creates a new automatically joining thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::{JoinHandle, JoinHandleWrapper};
    /// # use ina_threading::join::Join;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let handle = Join::new(JoinHandle::spawn(|| {
    ///     println!("Hello from another thread!");
    /// })?);
    ///
    /// // The thread is automatically joined.
    /// drop(handle);
    /// # Ok(())
    /// # }
    /// ```
    pub const fn new(handle: H) -> Self {
        Self { handle: Some(handle), first: None, value: None, panic: None }
    }

    /// Run the provided function before the thread is automatically joined.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::{JoinHandle, JoinHandleWrapper};
    /// # use ina_threading::join::Join;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let handle = Join::new(JoinHandle::spawn(|| ())?).first(|handle| {
    ///     let id = handle.as_join_handle().thread().id();
    ///
    ///     println!("the thread's identifier is: {id:?}");
    /// });
    ///
    /// // The thread is automatically joined.
    /// drop(handle);
    /// # Ok(())
    /// # }
    /// ```
    pub const fn first(mut self, f: fn(&mut H)) -> Self {
        self.first = Some(f);

        self
    }

    /// Run the provided function after the thread is automatically joined and its output is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::{JoinHandle, JoinHandleWrapper};
    /// # use ina_threading::join::Join;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let handle = Join::new(JoinHandle::spawn(|| 123)?)
    ///     .value(|value| println!("the output value was {value}!!!"));
    ///
    /// // The thread is automatically joined.
    /// drop(handle);
    /// # Ok(())
    /// # }
    /// ```
    pub const fn value(mut self, f: fn(H::Output)) -> Self {
        self.value = Some(f);

        self
    }

    /// Run the provided function after the thread is automatically joined and its panic value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ina_threading::{JoinHandle, JoinHandleWrapper};
    /// # use ina_threading::join::Join;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let handle = Join::new(JoinHandle::spawn(|| {
    ///     panic!("something went wrong!!!");
    /// })?)
    /// .panic(|_| println!("but it's okay!!!"));
    ///
    /// // The thread is automatically joined.
    /// drop(handle);
    /// # Ok(())
    /// # }
    /// ```
    pub const fn panic(mut self, f: fn(Box<dyn Any + Send + 'static>)) -> Self {
        self.panic = Some(f);

        self
    }
}

impl<H> AsRef<H> for Join<H>
where
    H: JoinHandleWrapper,
{
    #[inline]
    fn as_ref(&self) -> &H {
        self.handle.as_ref().unwrap_or_else(|| unreachable!("this is only `None` when the wrapper is dropped"))
    }
}

impl<H> Deref for Join<H>
where
    H: JoinHandleWrapper,
{
    type Target = H;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<H> AsMut<H> for Join<H>
where
    H: JoinHandleWrapper,
{
    #[inline]
    fn as_mut(&mut self) -> &mut H {
        self.handle.as_mut().unwrap_or_else(|| unreachable!("this is only `None` when the wrapper is dropped"))
    }
}

impl<H> DerefMut for Join<H>
where
    H: JoinHandleWrapper,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<H> Drop for Join<H>
where
    H: JoinHandleWrapper,
{
    fn drop(&mut self) {
        let Some(mut handle) = self.handle.take() else { return };

        if let Some(before) = self.first {
            before(&mut handle);
        }

        match handle.into_join_handle().join() {
            Ok(value) => self.value.unwrap_or(drop)(value),
            Err(value) => self.panic.unwrap_or(Self::DEFAULT_PANIC_FN)(value),
        }
    }
}
