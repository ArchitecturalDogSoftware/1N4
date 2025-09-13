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

use super::{DataReader, DataSystem, DataWriter};

/// The global instance of the file system.
static INSTANCE: RwLock<FileSystem> = RwLock::new(FileSystem);

/// A file-based data storage system.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileSystem;

#[expect(clippy::expect_used, reason = "a lock being poisoned means that there is potentially invalid state")]
impl DataSystem for FileSystem {
    fn get() -> impl Deref<Target = Self> {
        INSTANCE.read().expect("the lock has been poisoned")
    }

    fn get_mut() -> impl DerefMut<Target = Self> {
        INSTANCE.write().expect("the lock has been poisoned")
    }
}

impl DataReader for FileSystem {
    type Error = std::io::Error;

    fn exists(&self, path: &Path) -> Result<bool, Self::Error> {
        std::fs::exists(path)
    }

    fn size(&self, path: &Path) -> Result<u64, Self::Error> {
        Ok(std::fs::metadata(path)?.len())
    }

    fn read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error> {
        Ok(std::fs::read(path)?.into())
    }
}

impl DataWriter for FileSystem {
    type Error = std::io::Error;

    fn write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
        if let Some(path) = path.parent() {
            std::fs::create_dir_all(path)?;
        }

        std::fs::write(path, bytes)
    }

    fn rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error> {
        if let Some(path) = into.parent() {
            std::fs::create_dir_all(path)?;
        }

        std::fs::rename(from, into)
    }

    fn delete(&mut self, path: &Path) -> Result<(), Self::Error> {
        if std::fs::metadata(path)?.is_dir() { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) }
    }
}
