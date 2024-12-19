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

use std::path::Path;
use std::sync::Arc;

use ina_threading::statics::Static;
use ina_threading::threads::invoker::StatefulInvoker;
use tokio::sync::RwLock;

use crate::format::{DataDecode, DataEncode};
use crate::settings::Settings;
use crate::stored::Stored;
use crate::system::{DataReader, DataWriter};
use crate::{Result, Storage};

/// The storage thread's handle.
static THREAD: StorageThread = StorageThread::new();

/// The storage thread's type.
pub type StorageThread = Static<StatefulInvoker<Storage, Request, Response>, ()>;

/// A request sent to the storage thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Returns whether a path exists.
    Exists(Box<Path>),
    /// Returns the size of the data at the given path.
    Size(Box<Path>),
    /// Returns the data at the given path.
    Read(Box<Path>),
    /// Writes bytes into the given path.
    Write(Box<Path>, Arc<[u8]>),
    /// Renames the bytes to be associated with a new path.
    Rename(Box<Path>, Box<Path>),
    /// Deletes the data at the given path.
    Delete(Box<Path>),
}

/// A response sent from the storage thread.
#[derive(Debug)]
pub enum Response {
    /// Acknowledges a request.
    Acknowledge,
    /// Fails a request.
    Error(anyhow::Error),
    /// Whether data exists.
    Exists(bool),
    /// The size of some data.
    Size(u64),
    /// The bytes of some data.
    Read(Arc<[u8]>),
}

/// Starts the storage thread.
///
/// # Panics
///
/// Panics if the thread has already been initialized.
///
/// # Errors
///
/// This function will return an error if the thread fails to spawn.
pub async fn start(settings: Settings) -> Result<()> {
    assert!(!THREAD.async_api().has().await, "the thread has already been initialized");

    let capacity = settings.queue_capacity.get();
    let storage = Storage { settings };

    THREAD.async_api().set(StatefulInvoker::spawn_with_runtime("storage", storage, self::run, capacity)?).await;

    Ok(())
}

/// Starts the storage thread, blocking the current thread until successful.
///
/// # Panics
///
/// Panics if the thread has already been initialized or if this is called from within an asynchronous context.
///
/// # Errors
///
/// This function will return an error if the thread fails to spawn.
pub fn blocking_start(settings: Settings) -> Result<()> {
    assert!(!THREAD.sync_api().has(), "the thread has already been initialized");

    let capacity = settings.queue_capacity.get();
    let storage = Storage { settings };

    THREAD.sync_api().set(StatefulInvoker::spawn_with_runtime("storage", storage, self::run, capacity)?);

    Ok(())
}

/// Closes the storage thread.
///
/// # Panics
///
/// Panics if the storage thread is not initialized.
pub async fn close() {
    assert!(THREAD.async_api().has().await, "the thread is not initialized");

    THREAD.async_api().drop().await;
}

/// Closes the storage thread.
///
/// This blocks the current thread.
///
/// # Panics
///
/// Panics if the storage thread is not initialized or if this is called in an asynchronous context.
pub fn blocking_close() {
    assert!(THREAD.sync_api().has(), "the thread is not initialized");

    THREAD.sync_api().drop();
}

/// Runs the thread's process.
async fn run(storage: Arc<RwLock<Storage>>, input: Request) -> Response {
    match &input {
        Request::Exists(path) => storage.read().await.exists(path).await.map_or_else(Response::Error, Response::Exists),
        Request::Size(path) => storage.read().await.size(path).await.map_or_else(Response::Error, Response::Size),
        Request::Read(path) => storage.read().await.read(path).await.map_or_else(Response::Error, Response::Read),
        Request::Write(path, bytes) => {
            storage.write().await.write(path, bytes).await.map_or_else(Response::Error, |()| Response::Acknowledge)
        }
        Request::Rename(from, into) => {
            storage.write().await.rename(from, into).await.map_or_else(Response::Error, |()| Response::Acknowledge)
        }
        Request::Delete(path) => {
            storage.write().await.delete(path).await.map_or_else(Response::Error, |()| Response::Acknowledge)
        }
    }
}

/// Creates a thread invoker function.
macro_rules! invoke {
    ($(
        $(#[$attribute:meta])*
        $name:ident, $blocking_name:ident $(($($input:ident: $type:ty),*))? {
            $($request:tt)*
        } -> $return:ty {
            $($response:tt)*
        };
    )*) => {$(
        $(#[$attribute])*
        pub async fn $name($($($input: $type),*)?) -> anyhow::Result<$return> {
            let response = THREAD.async_api().get_mut().await.invoke($($request)*).await?;

            match response {
                $($response)*
                Response::Error(error) => Err(error),
                _ => unreachable!("unexpected response: '{response:?}'"),
            }
        }

        $(#[$attribute])*
        ///
        /// # Panics
        ///
        /// Panics if this is called from within a synchronous context.
        pub fn $blocking_name($($($input: $type),*)?) -> anyhow::Result<$return> {
            let response = THREAD.sync_api().get_mut().blocking_invoke($($request)*)?;

            match response {
                $($response)*
                Response::Error(error) => Err(error),
                _ => unreachable!("unexpected response: '{response:?}'"),
            }
        }
    )*};
}

invoke! {
    /// Returns whether data exists at the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    exists, blocking_exists (path: Box<Path>) {
        Request::Exists(path)
    } -> bool {
        Response::Exists(exists) => Ok(exists),
    };

    /// Returns the size of the data at the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    size, blocking_size (path: Box<Path>) {
        Request::Size(path)
    } -> u64 {
        Response::Size(size) => Ok(size),
    };

    /// Returns whether data exists at the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    rename, blocking_rename (from: Box<Path>, into: Box<Path>) {
        Request::Rename(from, into)
    } -> () {
        Response::Acknowledge => Ok(()),
    };

    /// Returns whether data exists at the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    delete, blocking_delete (path: Box<Path>) {
        Request::Delete(path)
    } -> () {
        Response::Acknowledge => Ok(()),
    };
}

/// Returns the data at the given path.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn read<T: Stored>(path: Box<Path>) -> anyhow::Result<T> {
    let response = THREAD.async_api().get_mut().await.invoke(Request::Read(path)).await?;

    match response {
        Response::Read(bytes) => tokio::runtime::Handle::current()
            .block_on(async move { T::data_format().decode(&bytes) })
            .map_err(Into::into),
        Response::Error(error) => Err(error),
        _ => unreachable!("unexpected response: '{response:?}'"),
    }
}

/// Returns the data at the given path.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
///
/// # Panics
///
/// Panics if this is called from within a synchronous context.
pub fn blocking_read<T: Stored>(path: Box<Path>) -> anyhow::Result<T> {
    let response = THREAD.sync_api().get_mut().blocking_invoke(Request::Read(path))?;

    match response {
        Response::Read(bytes) => T::data_format().decode(&bytes).map_err(Into::into),
        Response::Error(error) => Err(error),
        _ => unreachable!("unexpected response: '{response:?}'"),
    }
}

/// Writes bytes into the given path.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
pub async fn write<T: Stored>(path: Box<Path>, value: &T) -> anyhow::Result<()> {
    let bytes = tokio::runtime::Handle::current().block_on(async move { T::data_format().encode(value) })?;
    let response = THREAD.async_api().get_mut().await.invoke(Request::Write(path, bytes)).await?;

    match response {
        Response::Acknowledge => Ok(()),
        Response::Error(error) => Err(error),
        _ => unreachable!("unexpected response: '{response:?}'"),
    }
}

/// Writes bytes into the given path.
///
/// # Errors
///
/// This function will return an error if the message could not be sent.
///
/// # Panics
///
/// Panics if this is called from within a synchronous context.
pub fn blocking_write<T: Stored>(path: Box<Path>, value: &T) -> anyhow::Result<()> {
    let bytes = T::data_format().encode(value)?;
    let response = THREAD.sync_api().get_mut().blocking_invoke(Request::Write(path, bytes))?;

    match response {
        Response::Acknowledge => Ok(()),
        Response::Error(error) => Err(error),
        _ => unreachable!("unexpected response: '{response:?}'"),
    }
}
