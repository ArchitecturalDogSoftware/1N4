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
use std::sync::Arc;

use tokio::sync::RwLock;

use super::{DataReader, DataSystem, DataWriter};

/// The global instance of the file system.
static INSTANCE: RwLock<FileSystem> = RwLock::const_new(FileSystem);

/// A file-based data storage system.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileSystem;

impl DataSystem for FileSystem {
    #[inline]
    fn blocking_get() -> impl Deref<Target = Self> {
        INSTANCE.blocking_read()
    }

    #[inline]
    async fn get() -> impl Deref<Target = Self> {
        INSTANCE.read().await
    }

    #[inline]
    fn blocking_get_mut() -> impl DerefMut<Target = Self> {
        INSTANCE.blocking_write()
    }

    #[inline]
    async fn get_mut() -> impl DerefMut<Target = Self> {
        INSTANCE.write().await
    }
}

impl DataReader for FileSystem {
    type Error = std::io::Error;

    #[inline]
    fn blocking_exists(&self, path: &Path) -> Result<bool, Self::Error> {
        std::fs::exists(path)
    }

    #[inline]
    async fn exists(&self, path: &Path) -> Result<bool, Self::Error> {
        tokio::fs::try_exists(path).await
    }

    #[inline]
    fn blocking_size(&self, path: &Path) -> Result<u64, Self::Error> {
        Ok(std::fs::metadata(path)?.len())
    }

    #[inline]
    async fn size(&self, path: &Path) -> Result<u64, Self::Error> {
        Ok(tokio::fs::metadata(path).await?.len())
    }

    #[inline]
    fn blocking_read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error> {
        Ok(std::fs::read(path)?.into())
    }

    #[inline]
    async fn read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error> {
        Ok(tokio::fs::read(path).await?.into())
    }
}

impl DataWriter for FileSystem {
    type Error = std::io::Error;

    fn blocking_write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
        if let Some(path) = path.parent() {
            std::fs::create_dir_all(path)?;
        }

        std::fs::write(path, bytes)
    }

    async fn write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
        if let Some(path) = path.parent() {
            tokio::fs::create_dir_all(path).await?;
        }

        tokio::fs::write(path, bytes).await
    }

    fn blocking_rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error> {
        if let Some(path) = into.parent() {
            std::fs::create_dir_all(path)?;
        }

        std::fs::rename(from, into)
    }

    async fn rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error> {
        if let Some(path) = into.parent() {
            tokio::fs::create_dir_all(path).await?;
        }

        tokio::fs::rename(from, into).await
    }

    #[inline]
    fn blocking_delete(&mut self, path: &Path) -> Result<(), Self::Error> {
        if path.is_dir() { std::fs::remove_dir(path) } else { std::fs::remove_file(path) }
    }

    #[inline]
    async fn delete(&mut self, path: &Path) -> Result<(), Self::Error> {
        if path.is_dir() { tokio::fs::remove_dir(path).await } else { tokio::fs::remove_file(path).await }
    }
}
