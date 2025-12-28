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

impl DataReader for FileSystem {
    type Error = std::io::Error;

    fn blocking_exists(&self, path: &Path) -> Result<bool, Self::Error> {
        std::fs::exists(path)
    }

    async fn exists(&self, path: &Path) -> Result<bool, Self::Error> {
        tokio::fs::try_exists(path).await
    }

    fn blocking_size(&self, path: &Path) -> Result<u64, Self::Error> {
        Ok(std::fs::metadata(path)?.len())
    }

    async fn size(&self, path: &Path) -> Result<u64, Self::Error> {
        Ok(tokio::fs::metadata(path).await?.len())
    }

    fn blocking_read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error> {
        let mut file = std::fs::File::open(path)?;

        file.lock_shared()?;

        let file_size = file.metadata().map_or(0, |metadata| {
            // The vector may be at most `isize::MAX` bytes.
            usize::try_from(metadata.len()).unwrap_or(isize::MAX as usize)
        });
        let mut buffer = Vec::with_capacity(file_size);

        std::io::Read::read_to_end(&mut file, &mut buffer)?;

        file.unlock()?;

        Ok(buffer.into())
    }

    async fn read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error> {
        let file = tokio::fs::File::open(path).await?;

        // Currently, `lock` is not implemented in `tokio` due to the MSRV requirement.
        // Because of this, we need to juggle between the stdlib and tokio file types.
        let file = file.into_std().await;
        file.lock_shared()?;
        let mut file = tokio::fs::File::from_std(file);

        let file_size = file.metadata().await.map_or(0, |metadata| {
            // The vector may be at most `isize::MAX` bytes.
            usize::try_from(metadata.len()).unwrap_or(isize::MAX as usize)
        });
        let mut buffer = Vec::with_capacity(file_size);

        tokio::io::AsyncReadExt::read_to_end(&mut file, &mut buffer).await?;

        file.into_std().await.unlock()?;

        Ok(buffer.into())
    }
}

impl DataWriter for FileSystem {
    type Error = std::io::Error;

    fn blocking_write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
        if let Some(path) = path.parent() {
            std::fs::create_dir_all(path)?;
        }

        // We have to use `options` here because `File::create` will truncate before the lock is acquired.
        let mut file = if self.blocking_exists(path)? {
            std::fs::File::options().write(true).open(path)?
        } else {
            std::fs::File::options().create_new(true).write(true).open(path)?
        };

        file.lock()?;

        // Try to resize to match the length of the byte array, truncating to zero if the value is too large.
        // Realistically, since 128-bit systems are not commonplace, this is unnecessary and will always succeed.
        file.set_len(bytes.len().try_into().unwrap_or(0))?;

        std::io::Write::write_all(&mut file, bytes)?;

        file.unlock()
    }

    async fn write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
        if let Some(path) = path.parent() {
            tokio::fs::create_dir_all(path).await?;
        }

        // We have to use `options` here because `File::create` will truncate before the lock is acquired.
        let file = if self.exists(path).await? {
            tokio::fs::File::options().write(true).open(path).await?
        } else {
            tokio::fs::File::options().create_new(true).write(true).open(path).await?
        };

        // Currently, `lock` is not implemented in `tokio` due to the MSRV requirement.
        // Because of this, we need to juggle between the stdlib and tokio file types.
        let file = file.into_std().await;
        file.lock()?;
        let mut file = tokio::fs::File::from_std(file);

        // Try to resize to match the length of the byte array, truncating to zero if the value is too large.
        // Realistically, since 128-bit systems are not commonplace, this is unnecessary and will always succeed.
        file.set_len(bytes.len().try_into().unwrap_or(0)).await?;

        tokio::io::AsyncWriteExt::write_all(&mut file, bytes).await?;

        file.into_std().await.unlock()
    }

    fn blocking_rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error> {
        if let Some(path) = into.parent() {
            std::fs::create_dir_all(path)?;
        }

        if self.blocking_exists(into)? {
            let file = std::fs::File::open(into)?;

            // Acquire an exclusive lock on the file to ensure nothing else is currently using it.
            file.lock()?;
            // Then immediately drop it so that we can safely overwrite the file.
            file.unlock()?;
        }

        std::fs::rename(from, into)
    }

    async fn rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error> {
        if let Some(path) = into.parent() {
            tokio::fs::create_dir_all(path).await?;
        }

        if self.exists(into).await? {
            // Currently, `lock` is not implemented in `tokio` due to the MSRV requirement.
            // Because of this, we need to juggle between the stdlib and tokio file types.
            let file = tokio::fs::File::open(into).await?.into_std().await;

            // Acquire an exclusive lock on the file to ensure nothing else is currently using it.
            file.lock()?;
            // Then immediately drop it so that we can safely overwrite the file.
            file.unlock()?;
        }

        tokio::fs::rename(from, into).await
    }

    fn blocking_delete(&mut self, path: &Path) -> Result<(), Self::Error> {
        if std::fs::metadata(path)?.is_dir() { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) }
    }

    async fn delete(&mut self, path: &Path) -> Result<(), Self::Error> {
        if tokio::fs::metadata(path).await?.is_dir() {
            tokio::fs::remove_dir_all(path).await
        } else {
            tokio::fs::remove_file(path).await
        }
    }
}
