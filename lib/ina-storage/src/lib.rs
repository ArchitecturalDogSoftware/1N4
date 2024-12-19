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

//! Provides data storage solutions for 1N4.
#![feature(impl_trait_in_fn_trait_return)]

#[cfg(feature = "caching")]
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::Path;
use std::sync::Arc;

use clap::ValueEnum;
use ina_threading::threads::invoker::{Nonce, State};
use serde::{Deserialize, Serialize};
use thread::Request;
#[cfg(feature = "caching")]
use tokio::sync::RwLock;
use tokio::sync::mpsc::error::SendError;

use crate::settings::Settings;
use crate::system::{DataReader, DataSystem, DataWriter};

#[cfg(all(not(feature = "system-file"), not(feature = "system-memory")))]
compile_error!("at least one storage system feature must be enabled");

/// Defines data storage formats.
pub mod format;
/// Defines the storage system's settings.
pub mod settings;
/// Defines a trait for stored values.
pub mod stored;
/// Defines data storage systems.
pub mod system;
/// Defines the library's thread implementation.
pub mod thread;

/// A result alias with a defaulted error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error that may occur when using this library.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error<S = Infallible> {
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A sending error.
    #[error(transparent)]
    Send(#[from] SendError<S>),
    /// An error from communicating with a thread.
    #[error(transparent)]
    Thread(#[from] ina_threading::Error<Nonce<(State<Storage>, Request)>>),
}

/// A storage instance.
#[derive(Debug)]
pub struct Storage {
    /// The storage instance's settings.
    settings: Settings,
    /// The storage instance's internal cache.
    #[cfg(feature = "caching")]
    cache: RwLock<HashMap<Box<Path>, Arc<[u8]>>>,
}

impl Storage {
    /// Creates a new [`Storage`].
    #[cfg(not(feature = "caching"))]
    #[must_use]
    pub const fn new(settings: Settings) -> Self {
        Self { settings }
    }

    /// Creates a new [`Storage`].
    #[cfg(feature = "caching")]
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        Self { settings, cache: RwLock::new(HashMap::new()) }
    }
}

/// The preference for the storage backend system.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum System {
    /// The file system.
    #[cfg(feature = "system-file")]
    #[default]
    File,
    /// The memory system. This should only be used for testing, as data does not persist between runs.
    #[cfg(feature = "system-memory")]
    #[cfg_attr(not(feature = "system-file"), default)]
    Memory,
}

macro_rules! system_call {
    (match $system:expr, $($header:ident)* => $($call:tt)*) => {
        match $system {
            #[cfg(feature = "system-file")]
            System::File => system_call!($($header)* $crate::system::FileSystem => $($call)*),
            #[cfg(feature = "system-memory")]
            System::Memory => system_call!($($header)* $crate::system::MemorySystem => $($call)*),
        }
    };
    (async ref $type:ty => $($call:tt)*) => {
        <$type>::get().await$($call)*.await.map_err(Into::into)
    };
    (async mut $type:ty => $($call:tt)*) => {
        <$type>::get_mut().await$($call)*.await.map_err(Into::into)
    };
    (ref $type:ty => $($call:tt)*) => {
        <$type>::blocking_get()$($call)*.map_err(Into::into)
    };
    (mut $type:ty => $($call:tt)*) => {
        <$type>::blocking_get_mut()$($call)*.map_err(Into::into)
    };
}

impl DataReader for Storage {
    type Error = anyhow::Error;

    fn blocking_exists(&self, path: &Path) -> Result<bool, Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        if self.cache.blocking_read().contains_key(&(*path)) {
            return Ok(true);
        }

        system_call!(match self.settings.system, ref => .blocking_exists(&path))
    }

    async fn exists(&self, path: &Path) -> Result<bool, Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        if self.cache.read().await.contains_key(&(*path)) {
            return Ok(true);
        }

        system_call!(match self.settings.system, async ref => .exists(&path))
    }

    fn blocking_size(&self, path: &Path) -> Result<u64, Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        if let Some(bytes) = self.cache.blocking_read().get(&(*path)) {
            return Ok(bytes.len() as u64);
        }

        system_call!(match self.settings.system, ref => .blocking_size(&path))
    }

    async fn size(&self, path: &Path) -> Result<u64, Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        {
            let cache = self.cache.read().await;

            if let Some(bytes) = cache.get(&(*path)) {
                return Ok(bytes.len() as u64);
            }
        }

        system_call!(match self.settings.system, async ref => .size(&path))
    }

    fn blocking_read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        {
            let cache = self.cache.blocking_read();

            if let Some(bytes) = cache.get(&(*path)).cloned() {
                return Ok(bytes);
            }

            drop(cache);

            system_call!(match self.settings.system, ref => .blocking_read(&path)).inspect(|bytes| {
                self.cache.blocking_write().insert(path.into_boxed_path(), Arc::clone(bytes));
            })
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, ref => .blocking_read(&path))
        }
    }

    async fn read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        {
            let cache = self.cache.read().await;

            if let Some(bytes) = cache.get(&(*path)).cloned() {
                return Ok(bytes);
            }

            drop(cache);

            let result = system_call!(match self.settings.system, async ref => .read(&path));

            if let Ok(bytes) = result.as_ref().cloned() {
                self.cache.write().await.insert(path.into_boxed_path(), bytes);
            }

            result
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, async ref => .read(&path))
        }
    }
}

impl DataWriter for Storage {
    type Error = anyhow::Error;

    fn blocking_write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        {
            system_call!(match self.settings.system, mut => .blocking_write(&path, bytes)).inspect(|&()| {
                self.cache.blocking_write().insert(path.into_boxed_path(), Arc::from(bytes));
            })
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, mut => .blocking_write(&path, bytes))
        }
    }

    async fn write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        {
            let result = system_call!(match self.settings.system, async mut => .write(&path, bytes));

            if result.is_ok() {
                self.cache.write().await.insert(path.into_boxed_path(), Arc::from(bytes));
            }

            result
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, async mut => .write(&path, bytes))
        }
    }

    fn blocking_rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error> {
        let from = self.settings.directory.join(from);
        let into = self.settings.directory.join(into);

        #[cfg(feature = "caching")]
        {
            system_call!(match self.settings.system, mut => .blocking_rename(&from, &into)).inspect(|&()| {
                let mut cache = self.cache.blocking_write();
                let Some(value) = cache.remove(&(*from)) else { return };

                cache.insert(into.into_boxed_path(), value);
            })
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, mut => .blocking_rename(&from, &into))
        }
    }

    async fn rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error> {
        let from = self.settings.directory.join(from);
        let into = self.settings.directory.join(into);

        #[cfg(feature = "caching")]
        {
            let result = system_call!(match self.settings.system, async mut => .rename(&from, &into));

            if result.is_ok() {
                let mut cache = self.cache.blocking_write();
                let Some(value) = cache.remove(&(*from)) else { return result };

                cache.insert(into.into_boxed_path(), value);
            }

            result
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, async mut => .rename(&from, &into))
        }
    }

    fn blocking_delete(&mut self, path: &Path) -> Result<(), Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        {
            system_call!(match self.settings.system, mut => .blocking_delete(&path)).inspect(|&()| {
                self.cache.blocking_write().remove(&(*path));
            })
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, mut => .blocking_delete(&path))
        }
    }

    async fn delete(&mut self, path: &Path) -> Result<(), Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        {
            let result = system_call!(match self.settings.system, async mut => .delete(&path));

            if result.is_ok() {
                self.cache.write().await.remove(&(*path));
            }

            result
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, async mut => .delete(&path))
        }
    }
}
