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

use std::convert::Infallible;
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use system::DataSystem;
use tokio::sync::mpsc::error::SendError;

/// Defines data storage formats.
pub mod format;
/// Defines a trait for stored values.
pub mod stored;
/// Defines data storage systems.
pub mod system;

/// An error that may occur when using this library.
#[derive(Debug, thiserror::Error)]
pub enum Error<S = Infallible> {
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A threading error.
    #[error(transparent)]
    Threading(#[from] ina_threading::Error<S>),
    /// A sending error.
    #[error(transparent)]
    Send(#[from] SendError<S>),
}

/// The storage instance's settings.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Args, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[group(id = "DataSettings")]
pub struct Settings {
    /// The storage system to use to read and write data.
    #[arg(long = "data-system", default_value = "filesystem")]
    pub system: System,

    /// The directory within which to manage data files.
    #[arg(id = "DATA_DIRECTORY", long = "data-directory", default_value = "./res/data/")]
    #[serde(rename = "directory")]
    pub file_directory: Box<Path>,

    /// The storage thread's output queue capacity. If set to '1', no buffering will be done.
    #[arg(id = "DATA_QUEUE_CAPACITY", long = "data-queue-capacity", default_value = "8")]
    pub queue_capacity: NonZeroUsize,
}

/// The preference for the storage backend system.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum System {
    /// The file system.
    #[default]
    Filesystem,
}

impl System {
    /// Returns a reference to the instance of this system.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    #[must_use]
    pub fn blocking_get(self) -> impl Deref<Target = impl DataSystem> {
        match self {
            Self::Filesystem => crate::system::file::FileSystem::blocking_get(),
        }
    }

    /// Returns a reference to the instance of this system.
    pub async fn get(self) -> impl Deref<Target = impl DataSystem> {
        match self {
            Self::Filesystem => crate::system::file::FileSystem::get().await,
        }
    }

    /// Returns a mutable reference to the instance of this system.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    #[must_use]
    pub fn blocking_get_mut(self) -> impl DerefMut<Target = impl DataSystem> {
        match self {
            Self::Filesystem => crate::system::file::FileSystem::blocking_get_mut(),
        }
    }

    /// Returns a mutable reference to the instance of this system.
    pub async fn get_mut(self) -> impl DerefMut<Target = impl DataSystem> {
        match self {
            Self::Filesystem => crate::system::file::FileSystem::get_mut().await,
        }
    }
}

/// A storage instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Storage {
    /// The storage instance's setrtings.
    settings: Settings,
}
