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

use crate::Handle;

/// A thread that is automatically joined when dropped.
#[must_use = "this thread will drop and attempt to join immediately if left unused"]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Joining<H>
where
    H: Handle,
{
    /// The inner thread handle.
    handle: Option<H>,
    /// A function that is called before joining the thread.
    inspect_handle: Option<fn(&mut H)>,
    /// A function that is called after joining the thread.
    inspect_result: Option<fn(H::Output)>,
}

impl<H> Joining<H>
where
    H: Handle,
{
    /// Creates a new [`Joining<H, T>`] thread handle.
    pub const fn new(handle: H) -> Self {
        Self { handle: Some(handle), inspect_handle: None, inspect_result: None }
    }

    /// Runs the given function before joining the thread on drop.
    pub const fn inspect_handle(mut self, f: fn(&mut H)) -> Self {
        self.inspect_handle = Some(f);

        self
    }

    /// Runs the given function after joining the thread on drop.
    pub const fn inspect_result(mut self, f: fn(H::Output)) -> Self {
        self.inspect_result = Some(f);

        self
    }
}

impl<H> Deref for Joining<H>
where
    H: Handle,
{
    type Target = H;

    #[expect(clippy::expect_used, reason = "the thread should not be accessed if it has been joined")]
    fn deref(&self) -> &Self::Target {
        self.handle.as_ref().expect("attempted to access a dropped thread")
    }
}

impl<H> DerefMut for Joining<H>
where
    H: Handle,
{
    #[expect(clippy::expect_used, reason = "the thread should not be accessed if it has been joined")]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.handle.as_mut().expect("attempted to access a dropped thread")
    }
}

impl<H> Drop for Joining<H>
where
    H: Handle,
{
    #[expect(clippy::unwrap_used, reason = "panics should be bubbled up since we can't drop with a result")]
    fn drop(&mut self) {
        let Some(mut handle) = self.handle.take() else { return };

        if let Some(inspect_handle) = self.inspect_handle {
            inspect_handle(&mut handle);
        }

        let result = handle.into_join_handle().join().unwrap();

        self.inspect_result.unwrap_or(drop)(result);
    }
}
