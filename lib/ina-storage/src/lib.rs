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
use std::fmt::Display;
#[cfg(feature = "caching")]
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::Arc;
#[cfg(feature = "caching")]
use std::sync::RwLock;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::settings::Settings;
use crate::system::{DataReader, DataSystem, DataWriter};
use crate::thread::JoinHandle;

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
pub enum Error {
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An error related to the static thread handle.
    #[error(transparent)]
    Thread(#[from] ina_threading::statics::Error<JoinHandle>),
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
    #[cfg_attr(
        not(feature = "caching"),
        expect(
            clippy::missing_const_for_fn,
            reason = "this function cannot be const with the `caching` feature enabled"
        )
    )]
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        #[cfg(feature = "caching")]
        {
            Self { settings, cache: RwLock::new(HashMap::new()) }
        }
        #[cfg(not(feature = "caching"))]
        {
            Self { settings }
        }
    }

    /// Returns an immutable reference to the storage cache.
    #[cfg(feature = "caching")]
    pub(crate) fn cache_read(&self) -> impl Deref<Target = HashMap<Box<Path>, Arc<[u8]>>> {
        if self.cache.is_poisoned() {
            // If the cache is poisoned, we have to assume that it contains potentially faulty data.
            self.cache.clear_poison();
            self.cache.write().unwrap_or_else(|_| unreachable!("we just cleared the poison")).clear();
        }

        self.cache.read().unwrap_or_else(|_| unreachable!("the poison is guaranteed to be cleared at this point"))
    }

    /// Returns an immutable reference to the storage cache.
    #[cfg(feature = "caching")]
    pub(crate) fn cache_write(&self) -> impl DerefMut<Target = HashMap<Box<Path>, Arc<[u8]>>> {
        if self.cache.is_poisoned() {
            // If the cache is poisoned, we have to assume that it contains potentially faulty data.
            self.cache.clear_poison();

            let mut lock = self.cache.write().unwrap_or_else(|_| unreachable!("we just cleared the poison"));
            lock.clear();
            lock
        } else {
            self.cache.write().unwrap_or_else(|_| unreachable!("the lock cannot be poisoned"))
        }
    }
}

/// The preference for the storage backend system.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum System {
    /// The file system.
    #[cfg(feature = "system-file")]
    #[cfg_attr(feature = "system-file", default)]
    File,
    /// The memory system. This should only be used for testing, as data does not persist between runs.
    #[cfg(feature = "system-memory")]
    #[cfg_attr(not(feature = "system-file"), default)]
    Memory,
}

impl Display for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some(value) = self.to_possible_value() else { unreachable!("no variants are marked as skipped") };

        f.write_str(value.get_name())
    }
}

/// Calls a function provided by the [`DataSystem`] trait by matching the given [`System`] enum's variant(s).
macro_rules! system_call {
    (match $system:expr, $($header:ident)* => $($call:tt)*) => {
        match $system {
            #[cfg(feature = "system-file")]
            System::File => system_call!($($header)* $crate::system::FileSystem => $($call)*),
            #[cfg(feature = "system-memory")]
            System::Memory => system_call!($($header)* $crate::system::MemorySystem => $($call)*),
        }
    };
    (ref $type:ty => $($call:tt)*) => {
        <$type>::get()$($call)*.map_err(Into::into)
    };
    (mut $type:ty => $($call:tt)*) => {
        <$type>::get_mut()$($call)*.map_err(Into::into)
    };
}

impl DataReader for Storage {
    type Error = anyhow::Error;

    fn exists(&self, path: &Path) -> Result<bool, Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        if self.cache_read().contains_key(&(*path)) {
            return Ok(true);
        }

        system_call!(match self.settings.system, ref => .exists(&path))
    }

    fn size(&self, path: &Path) -> Result<u64, Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        if let Some(bytes) = self.cache_read().get(&(*path)) {
            return Ok(bytes.len() as u64);
        }

        system_call!(match self.settings.system, ref => .size(&path))
    }

    fn read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        {
            let cache = self.cache_read();

            if let Some(bytes) = cache.get(&(*path)).cloned() {
                return Ok(bytes);
            }

            drop(cache);

            system_call!(match self.settings.system, ref => .read(&path)).inspect(|bytes| {
                self.cache_write().insert(path.into_boxed_path(), Arc::clone(bytes));
            })
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, ref => .read(&path))
        }
    }
}

impl DataWriter for Storage {
    type Error = anyhow::Error;

    fn write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        {
            system_call!(match self.settings.system, mut => .write(&path, bytes)).inspect(|&()| {
                self.cache_write().insert(path.into_boxed_path(), Arc::from(bytes));
            })
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, mut => .write(&path, bytes))
        }
    }

    fn rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error> {
        let from = self.settings.directory.join(from);
        let into = self.settings.directory.join(into);

        #[cfg(feature = "caching")]
        {
            system_call!(match self.settings.system, mut => .rename(&from, &into)).inspect(|&()| {
                let mut cache = self.cache_write();
                let Some(value) = cache.remove(&(*from)) else { return };

                cache.insert(into.into_boxed_path(), value);
            })
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, mut => .rename(&from, &into))
        }
    }

    fn delete(&mut self, path: &Path) -> Result<(), Self::Error> {
        let path = self.settings.directory.join(path);

        #[cfg(feature = "caching")]
        {
            system_call!(match self.settings.system, mut => .delete(&path)).inspect(|&()| {
                self.cache_write().remove(&(*path));
            })
        }
        #[cfg(not(feature = "caching"))]
        {
            system_call!(match self.settings.system, mut => .delete(&path))
        }
    }
}
