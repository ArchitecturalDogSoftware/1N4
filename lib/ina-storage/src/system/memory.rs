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

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::{Arc, LazyLock};

use tokio::sync::RwLock;

use super::{DataReader, DataSystem, DataWriter};

/// The global instance of the memory system.
static INSTANCE: LazyLock<RwLock<MemorySystem>> = LazyLock::new(RwLock::default);

/// An error that can be returned by the memory system.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The path is missing from the system.
    #[error("missing path '{0}'")]
    MissingPath(Box<Path>),
}

/// A memory-based data storage system.
///
/// This should only ever be used for testing purposes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemorySystem {
    /// The inner hash map.
    inner: HashMap<Box<Path>, Arc<[u8]>>,
}

impl DataSystem for MemorySystem {
    fn blocking_get() -> impl Deref<Target = Self> {
        INSTANCE.blocking_read()
    }

    async fn get() -> impl Deref<Target = Self> {
        INSTANCE.read().await
    }

    fn blocking_get_mut() -> impl DerefMut<Target = Self> {
        INSTANCE.blocking_write()
    }

    async fn get_mut() -> impl DerefMut<Target = Self> {
        INSTANCE.write().await
    }
}

impl DataReader for MemorySystem {
    type Error = Error;

    fn blocking_exists(&self, path: &Path) -> Result<bool, Self::Error> {
        Ok(self.inner.contains_key(path))
    }

    async fn exists(&self, path: &Path) -> Result<bool, Self::Error> {
        Ok(self.inner.contains_key(path))
    }

    fn blocking_size(&self, path: &Path) -> Result<u64, Self::Error> {
        let Some(value) = self.inner.get(path) else {
            return Err(Error::MissingPath(path.into()));
        };

        Ok(value.len() as u64)
    }

    async fn size(&self, path: &Path) -> Result<u64, Self::Error> {
        self.blocking_size(path)
    }

    fn blocking_read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error> {
        self.inner.get(path).cloned().ok_or_else(|| Error::MissingPath(path.into()))
    }

    async fn read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error> {
        self.blocking_read(path)
    }
}

impl DataWriter for MemorySystem {
    type Error = Error;

    fn blocking_write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.insert(path.into(), bytes.into());

        Ok(())
    }

    async fn write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
        self.blocking_write(path, bytes)
    }

    fn blocking_rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error> {
        let Some(value) = self.inner.remove(from) else {
            return Err(Error::MissingPath(from.into()));
        };

        self.inner.insert(into.into(), value);

        Ok(())
    }

    async fn rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error> {
        self.blocking_rename(from, into)
    }

    fn blocking_delete(&mut self, path: &Path) -> Result<(), Self::Error> {
        if !self.blocking_exists(path)? {
            return Err(Error::MissingPath(path.into()));
        }

        self.inner.remove(path);

        Ok(())
    }

    async fn delete(&mut self, path: &Path) -> Result<(), Self::Error> {
        self.blocking_delete(path)
    }
}
