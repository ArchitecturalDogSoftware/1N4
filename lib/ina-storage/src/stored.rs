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
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::format::DataFormat;

/// A value that is storable within the storage system.
pub trait Stored: Serialize + for<'de> Deserialize<'de> {
    /// Returns a new instance of this value's expected data format.
    fn data_format(&self) -> impl DataFormat;

    /// Returns the expected storage path for this value.
    fn data_path(&self) -> impl AsRef<Path>;

    /// Returns an asynchronous API for this stored value type.
    #[inline]
    fn async_api() -> AsyncApi<Self> {
        AsyncApi(PhantomData)
    }

    /// Returns a synchronous API for this stored value type.
    #[inline]
    fn sync_api() -> SyncApi<Self> {
        SyncApi(PhantomData)
    }

    /// Returns an asynchronous API for this stored value type.
    #[inline]
    fn as_async_api(&self) -> AsyncHolderApi<Self> {
        Self::async_api().with(self)
    }

    /// Returns a synchronous API for this stored value type.
    #[inline]
    fn as_sync_api(&self) -> SyncHolderApi<Self> {
        Self::sync_api().with(self)
    }
}

/// An asynchronous API for a stored value type.
#[must_use = "api values do nothing unless used"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AsyncApi<T: Stored>(PhantomData<T>);

impl<T: Stored> AsyncApi<T> {
    /// Creates an asynchronous API that holds the given value.
    #[inline]
    pub const fn with(self, value: &T) -> AsyncHolderApi<T> {
        AsyncHolderApi(value)
    }
}

/// An asynchronous API for a held stored value.
#[must_use = "api values do nothing unless used"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AsyncHolderApi<'sv, T: Stored>(&'sv T);

impl<'sv, T: Stored> AsyncHolderApi<'sv, T> {}

/// A synchronous API for a stored value type.
#[must_use = "api values do nothing unless used"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SyncApi<T: Stored>(PhantomData<T>);

impl<T: Stored> SyncApi<T> {
    /// Creates a synchronous API that holds the given value.
    #[inline]
    pub const fn with(self, value: &T) -> SyncHolderApi<T> {
        SyncHolderApi(value)
    }
}

/// A synchronous API for a held stored value.
#[must_use = "api values do nothing unless used"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SyncHolderApi<'sv, T: Stored>(&'sv T);

impl<'sv, T: Stored> SyncHolderApi<'sv, T> {}
