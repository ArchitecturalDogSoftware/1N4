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

use std::ops::{Deref, DerefMut};

use crate::HandleHolder;

/// A thread that is automatically joined when dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
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
