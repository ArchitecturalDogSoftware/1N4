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

use std::sync::Arc;

pub use self::file::*;
pub use self::terminal::*;
use crate::entry::Entry;
use crate::settings::Settings;
use crate::{Error, Result};

/// The file endpoint implementation.
#[cfg(feature = "file")]
mod file;
/// The terminal endpoint implementation.
#[cfg(feature = "terminal")]
mod terminal;

/// Allows a type to be used as a logger output endpoint.
#[async_trait::async_trait]
pub trait Endpoint: std::fmt::Debug + Send + Sync + 'static {
    /// Returns this endpoint's internal name.
    fn name(&self) -> &'static str;

    /// Sets up and initializes this endpoint.
    ///
    /// # Errors
    ///
    /// This function will return an error if the endpoint could not be set up.
    async fn setup(&mut self, settings: &Settings) -> Result<()>;

    /// Writes the given entry into this endpoint.
    ///
    /// # Errors
    ///
    /// This function will return an error if the entry could not be written.
    async fn write(&mut self, entry: Arc<Entry<'static>>) -> Result<()>;

    /// Closes this endpoint.
    ///
    /// # Errors
    ///
    /// This function will return an error if the endpoint could not be closed.
    async fn close(&mut self) -> Result<()>;

    /// Returns an invalid state error.
    #[inline]
    fn invalid_state(&self) -> Error {
        Error::InvalidEndpointState(self.name())
    }
}
