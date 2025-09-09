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
use std::path::Path;
use std::sync::{Arc, RwLock};

use ina_threading::join::Join;
use ina_threading::statics::Static;
use ina_threading::threads::callable::StatefulCallableJoinHandle;

use crate::format::{DataDecode, DataEncode};
use crate::settings::Settings;
use crate::stored::Stored;
use crate::system::{DataReader, DataWriter};
use crate::{Result, Storage};

/// The storage thread's static handle.
static HANDLE: Static<JoinHandle> = Static::new();

/// The inner type of the thread's handle.
pub(crate) type JoinHandle = Join<StatefulCallableJoinHandle<Request, Response, RwLock<Storage>>>;

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
/// # Errors
///
/// This function will return an error if the thread fails to spawn.
pub async fn start(settings: Settings) -> Result<()> {
    let capacity = settings.queue_capacity;
    let state = Arc::new(RwLock::new(Storage::new(settings)));
    let handle = Join::new(StatefulCallableJoinHandle::spawn(capacity, state, self::run)?);

    HANDLE.initialize(handle).await.map_err(Into::into)
}

/// Closes the storage thread.
pub async fn close() {
    HANDLE.uninitialize().await;
}

/// Runs the thread's process.
fn run((state, request): (Arc<RwLock<Storage>>, Request)) -> Response {
    #[inline]
    fn read(state: &Arc<RwLock<Storage>>) -> impl Deref<Target = Storage> + '_ {
        assert!(!state.is_poisoned(), "storage was poisoned, possibly leading to corrupted data");

        state.read().unwrap_or_else(|_| unreachable!("the lock is guaranteed to not be poisoned"))
    }

    #[inline]
    fn write(state: &Arc<RwLock<Storage>>) -> impl DerefMut<Target = Storage> + '_ {
        assert!(!state.is_poisoned(), "storage was poisoned, possibly leading to corrupted data");

        state.write().unwrap_or_else(|_| unreachable!("the lock is guaranteed to not be poisoned"))
    }

    match &request {
        Request::Exists(path) => read(&state).exists(path).map_or_else(Response::Error, Response::Exists),
        Request::Size(path) => read(&state).size(path).map_or_else(Response::Error, Response::Size),
        Request::Read(path) => read(&state).read(path).map_or_else(Response::Error, Response::Read),
        Request::Write(path, bytes) => {
            write(&state).write(path, bytes).map_or_else(Response::Error, |()| Response::Acknowledge)
        }
        Request::Rename(from, into) => {
            write(&state).rename(from, into).map_or_else(Response::Error, |()| Response::Acknowledge)
        }
        Request::Delete(path) => write(&state).delete(path).map_or_else(Response::Error, |()| Response::Acknowledge),
    }
}

/// Creates a thread invocation function.
macro_rules! invoke {
    ($(
        $(#[$attribute:meta])*
        $name:ident$(($($input:ident: $type:ty),*))? {
            $($request:tt)*
        } -> $return:ty {
            $($response:tt)*
        };
    )*) => {$(
        $(#[$attribute])*
        pub async fn $name($($($input: $type),*)?) -> anyhow::Result<$return> {
            let response = HANDLE.try_get_mut().await?.invoke($($request)*).await?;

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
    exists(path: Box<Path>) {
        Request::Exists(path)
    } -> bool {
        Response::Exists(exists) => Ok(exists),
    };

    /// Returns the size of the data at the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    size(path: Box<Path>) {
        Request::Size(path)
    } -> u64 {
        Response::Size(size) => Ok(size),
    };

    /// Returns whether data exists at the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    rename(from: Box<Path>, into: Box<Path>) {
        Request::Rename(from, into)
    } -> () {
        Response::Acknowledge => Ok(()),
    };

    /// Returns whether data exists at the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the message could not be sent.
    delete(path: Box<Path>) {
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
    let response = HANDLE.try_get_mut().await?.invoke(Request::Read(path)).await?;

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
    let bytes = T::data_format().encode(value)?;
    let response = HANDLE.try_get_mut().await?.invoke(Request::Write(path, bytes)).await?;

    match response {
        Response::Acknowledge => Ok(()),
        Response::Error(error) => Err(error),
        _ => unreachable!("unexpected response: '{response:?}'"),
    }
}
