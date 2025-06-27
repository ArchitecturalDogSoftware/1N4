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

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::format::DataFormat;

/// A value that can be stored within the storage system.
pub trait Stored: Send + Sync + Serialize + for<'de> Deserialize<'de> {
    /// The arguments required to construct a new path for this type.
    type PathArguments: Send;

    /// Returns a new instance of this type's expected data format.
    fn data_format() -> impl DataFormat + Send;

    /// Returns the expected storage path for this type.
    fn data_path_for(arguments: Self::PathArguments) -> impl AsRef<Path> + Send;

    /// Returns the expected storage path for this value.
    fn data_path(&self) -> impl AsRef<Path> + Send;

    /// Returns an asynchronous API for this stored value type.
    fn async_api() -> AsyncApi<Self> {
        AsyncApi(PhantomData)
    }

    /// Returns a synchronous API for this stored value type.
    fn sync_api() -> SyncApi<Self> {
        SyncApi(PhantomData)
    }

    /// Returns an asynchronous API for this stored value type.
    fn as_async_api(&self) -> AsyncHolderApi<'_, Self> {
        Self::async_api().with(self)
    }

    /// Returns a synchronous API for this stored value type.
    fn as_sync_api(&self) -> SyncHolderApi<'_, Self> {
        Self::sync_api().with(self)
    }
}

/// An asynchronous API for a stored value type.
#[repr(transparent)]
#[must_use = "api values do nothing unless used"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AsyncApi<T: Stored>(PhantomData<T>);

impl<T: Stored> AsyncApi<T> {
    /// Creates an asynchronous API that holds the given value.
    pub const fn with(self, value: &T) -> AsyncHolderApi<'_, T> {
        AsyncHolderApi(value)
    }

    /// Returns whether data is stored for the value represented by the given path arguments.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub async fn exists(self, arguments: T::PathArguments) -> Result<bool> {
        let format = T::data_format();
        let path = T::data_path_for(arguments).as_ref().with_extension(format.extension());

        crate::thread::exists(path.into_boxed_path()).await
    }

    /// Returns the size of the stored data for the value represented by the given path arguments.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub async fn size(self, arguments: T::PathArguments) -> Result<u64> {
        let format = T::data_format();
        let path = T::data_path_for(arguments).as_ref().with_extension(format.extension());

        crate::thread::size(path.into_boxed_path()).await
    }

    /// Returns the stored value represented by the given path arguments.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub async fn read(self, arguments: T::PathArguments) -> Result<T> {
        let format = T::data_format();
        let path = T::data_path_for(arguments).as_ref().with_extension(format.extension());

        crate::thread::read(path.into_boxed_path()).await
    }

    /// Writes the given value into the storage system at the path represented by the given path arguments.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub async fn write(self, arguments: T::PathArguments, value: &T) -> Result<()> {
        let format = T::data_format();
        let path = T::data_path_for(arguments).as_ref().with_extension(format.extension());

        crate::thread::write(path.into_boxed_path(), value).await
    }

    /// Renames the value represented by the given path arguments.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub async fn rename(self, from: T::PathArguments, into: T::PathArguments) -> Result<()> {
        let format = T::data_format();
        let from = T::data_path_for(from).as_ref().with_extension(format.extension());
        let into = T::data_path_for(into).as_ref().with_extension(format.extension());

        crate::thread::rename(from.into_boxed_path(), into.into_boxed_path()).await
    }

    /// Deletes the value represented by the given path arguments.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub async fn delete(self, arguments: T::PathArguments) -> Result<()> {
        let format = T::data_format();
        let path = T::data_path_for(arguments).as_ref().with_extension(format.extension());

        crate::thread::delete(path.into_boxed_path()).await
    }
}

/// An asynchronous API for a held stored value.
#[repr(transparent)]
#[must_use = "api values do nothing unless used"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AsyncHolderApi<'sv, T: Stored>(&'sv T);

impl<T: Stored> AsyncHolderApi<'_, T> {
    /// Returns whether data is stored for this value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub async fn exists(self) -> Result<bool> {
        let format = T::data_format();
        let path = self.0.data_path().as_ref().with_extension(format.extension());

        crate::thread::exists(path.into_boxed_path()).await
    }

    /// Returns the size of the stored data for this value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub async fn size(self) -> Result<u64> {
        let format = T::data_format();
        let path = self.0.data_path().as_ref().with_extension(format.extension());

        crate::thread::size(path.into_boxed_path()).await
    }

    /// Returns the value as saved within the storage system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub async fn read(self) -> Result<T> {
        let format = T::data_format();
        let path = self.0.data_path().as_ref().with_extension(format.extension());

        crate::thread::read(path.into_boxed_path()).await
    }

    /// Writes this value into the storage system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub async fn write(self) -> Result<()> {
        let format = T::data_format();
        let path = self.0.data_path().as_ref().with_extension(format.extension());

        crate::thread::write(path.into_boxed_path(), self.0).await
    }

    /// Renames this value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub async fn rename(self, into: T::PathArguments) -> Result<()> {
        let format = T::data_format();
        let from = self.0.data_path().as_ref().with_extension(format.extension());
        let into = T::data_path_for(into).as_ref().with_extension(format.extension());

        crate::thread::rename(from.into_boxed_path(), into.into_boxed_path()).await
    }

    /// Deletes this value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub async fn delete(self) -> Result<()> {
        let format = T::data_format();
        let path = self.0.data_path().as_ref().with_extension(format.extension());

        crate::thread::delete(path.into_boxed_path()).await
    }
}

/// A synchronous API for a stored value type.
#[repr(transparent)]
#[must_use = "api values do nothing unless used"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SyncApi<T: Stored>(PhantomData<T>);

impl<T: Stored> SyncApi<T> {
    /// Creates a synchronous API that holds the given value.
    pub const fn with(self, value: &T) -> SyncHolderApi<'_, T> {
        SyncHolderApi(value)
    }

    /// Returns whether data is stored for the value represented by the given path arguments.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub fn exists(self, arguments: T::PathArguments) -> Result<bool> {
        let format = T::data_format();
        let path = T::data_path_for(arguments).as_ref().with_extension(format.extension());

        crate::thread::blocking_exists(path.into_boxed_path())
    }

    /// Returns the size of the stored data for the value represented by the given path arguments.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub fn size(self, arguments: T::PathArguments) -> Result<u64> {
        let format = T::data_format();
        let path = T::data_path_for(arguments).as_ref().with_extension(format.extension());

        crate::thread::blocking_size(path.into_boxed_path())
    }

    /// Returns the stored value represented by the given path arguments.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub fn read(self, arguments: T::PathArguments) -> Result<T> {
        let format = T::data_format();
        let path = T::data_path_for(arguments).as_ref().with_extension(format.extension());

        crate::thread::blocking_read(path.into_boxed_path())
    }

    /// Writes the given value into the storage system at the path represented by the given path arguments.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub fn write(self, arguments: T::PathArguments, value: &T) -> Result<()> {
        let format = T::data_format();
        let path = T::data_path_for(arguments).as_ref().with_extension(format.extension());

        crate::thread::blocking_write(path.into_boxed_path(), value)
    }

    /// Renames the value represented by the given path arguments.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub fn rename(self, from: T::PathArguments, into: T::PathArguments) -> Result<()> {
        let format = T::data_format();
        let from = T::data_path_for(from).as_ref().with_extension(format.extension());
        let into = T::data_path_for(into).as_ref().with_extension(format.extension());

        crate::thread::blocking_rename(from.into_boxed_path(), into.into_boxed_path())
    }

    /// Deletes the value represented by the given path arguments.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub fn delete(self, arguments: T::PathArguments) -> Result<()> {
        let format = T::data_format();
        let path = T::data_path_for(arguments).as_ref().with_extension(format.extension());

        crate::thread::blocking_delete(path.into_boxed_path())
    }
}

/// A synchronous API for a held stored value.
#[repr(transparent)]
#[must_use = "api values do nothing unless used"]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SyncHolderApi<'sv, T: Stored>(&'sv T);

impl<T: Stored> SyncHolderApi<'_, T> {
    /// Returns whether data is stored for this value.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub fn exists(self) -> Result<bool> {
        let format = T::data_format();
        let path = self.0.data_path().as_ref().with_extension(format.extension());

        crate::thread::blocking_exists(path.into_boxed_path())
    }

    /// Returns the size of the stored data for this value.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub fn size(self) -> Result<u64> {
        let format = T::data_format();
        let path = self.0.data_path().as_ref().with_extension(format.extension());

        crate::thread::blocking_size(path.into_boxed_path())
    }

    /// Returns the value as saved within the storage system.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    pub fn read(self) -> Result<T> {
        let format = T::data_format();
        let path = self.0.data_path().as_ref().with_extension(format.extension());

        crate::thread::blocking_read(path.into_boxed_path())
    }

    /// Writes this value into the storage system.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub fn write(self) -> Result<()> {
        let format = T::data_format();
        let path = self.0.data_path().as_ref().with_extension(format.extension());

        crate::thread::blocking_write(path.into_boxed_path(), self.0)
    }

    /// Renames this value.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub fn rename(self, into: T::PathArguments) -> Result<()> {
        let format = T::data_format();
        let from = self.0.data_path().as_ref().with_extension(format.extension());
        let into = T::data_path_for(into).as_ref().with_extension(format.extension());

        crate::thread::blocking_rename(from.into_boxed_path(), into.into_boxed_path())
    }

    /// Deletes this value.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    pub fn delete(self) -> Result<()> {
        let format = T::data_format();
        let path = self.0.data_path().as_ref().with_extension(format.extension());

        crate::thread::blocking_delete(path.into_boxed_path())
    }
}
