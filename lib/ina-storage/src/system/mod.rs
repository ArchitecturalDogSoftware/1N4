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

#[cfg(feature = "system-file")]
pub use self::file::FileSystem;
#[cfg(feature = "system-memory")]
pub use self::memory::MemorySystem;

/// A file-based system.
#[cfg(feature = "system-file")]
pub mod file;
/// A memory-based system. This should only ever be used for testing.
#[cfg(feature = "system-memory")]
pub mod memory;

/// A value that reads and writes generic data.
pub trait DataSystem: DataReader + DataWriter + 'static {
    /// Returns a reference to the instance of this system.
    fn get() -> impl Deref<Target = Self>;

    /// Returns a mutable reference to the instance of this system.
    fn get_mut() -> impl DerefMut<Target = Self>;
}

/// A value that reads data bytes.
pub trait DataReader {
    /// The error that can be returned during reading.
    type Error: Into<anyhow::Error>;

    /// Returns whether the path exists within this reader.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    fn exists(&self, path: &Path) -> Result<bool, Self::Error>;

    /// Returns the size of the data at the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    fn size(&self, path: &Path) -> Result<u64, Self::Error>;

    /// Reads bytes from the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be read.
    fn read(&self, path: &Path) -> Result<Arc<[u8]>, Self::Error>;
}

/// A value that writes data bytes.
pub trait DataWriter {
    /// The error that can be returned during writing.
    type Error: Into<anyhow::Error>;

    /// Writes bytes into the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    fn write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error>;

    /// Renames the bytes to be associated with a new path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    fn rename(&mut self, from: &Path, into: &Path) -> Result<(), Self::Error>;

    /// Deletes bytes from the given path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path cannot be written to.
    fn delete(&mut self, path: &Path) -> Result<(), Self::Error>;
}
