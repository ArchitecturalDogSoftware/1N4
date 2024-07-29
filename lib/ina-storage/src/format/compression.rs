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

use std::ffi::OsStr;
use std::fmt::Debug;
use std::io::Read;
use std::sync::Arc;

use flate2::read::{GzDecoder, GzEncoder};
use flate2::Compression;
use serde::{Deserialize, Serialize};

use super::{DataDecode, DataEncode, DataFormat};

/// A compression format error.
#[derive(Debug, thiserror::Error)]
pub enum Error<F: Debug + DataFormat> {
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An encoding error.
    #[error(transparent)]
    Encode(<F as DataEncode>::Error),
    /// A decoding error.
    #[error(transparent)]
    Decode(<F as DataDecode>::Error),
}

/// Compresses the wrapped format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Compress<F: Debug + DataFormat> {
    /// The inner format.
    inner: F,
    /// The compression level.
    level: Compression,
}

impl<F: Debug + DataFormat> Compress<F> {
    /// Creates a new [`Compress<F>`] format.
    ///
    /// The given level should be within the range `0..=9`.
    #[inline]
    pub const fn new(inner: F, level: u8) -> Self {
        Self { inner, level: Compression::new(level as u32) }
    }

    /// Creates a new [`Compress<F>`] format with no compression.
    #[inline]
    pub const fn new_none(inner: F) -> Self {
        Self::new(inner, 0)
    }

    /// Creates a new [`Compress<F>`] format using a fast level of compression.
    #[inline]
    pub const fn new_fast(inner: F) -> Self {
        Self::new(inner, 1)
    }

    /// Creates a new [`Compress<F>`] format using the default level of compression.
    #[inline]
    pub const fn new_default(inner: F) -> Self {
        Self::new(inner, 5)
    }

    /// Creates a new [`Compress<F>`] format using the best level of compression.
    #[inline]
    pub const fn new_best(inner: F) -> Self {
        Self::new(inner, 9)
    }
}

impl<F: Debug + DataFormat + 'static> DataFormat for Compress<F> {
    fn extension(&self) -> impl AsRef<OsStr> {
        format!("{}.gz", self.inner.extension().as_ref().to_string_lossy())
    }
}

impl<F: Debug + DataFormat + 'static> DataEncode for Compress<F> {
    type Error = Error<F>;

    fn encode<T: Serialize>(&self, value: &T) -> Result<Arc<[u8]>, Self::Error> {
        let bytes = self.inner.encode(value).map_err(Error::Encode)?;
        let mut encoder = GzEncoder::new(&(*bytes), self.level);
        let mut buffer = Vec::with_capacity(bytes.len());

        encoder.read_to_end(&mut buffer)?;

        Ok(buffer.into())
    }
}

impl<F: Debug + DataFormat + 'static> DataDecode for Compress<F> {
    type Error = Error<F>;

    fn decode<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> Result<T, Self::Error> {
        let mut decoder = GzDecoder::new(bytes);
        let mut buffer = Vec::with_capacity(bytes.len() * 3);

        decoder.read_to_end(&mut buffer)?;

        self.inner.decode(&buffer).map_err(Error::Decode)
    }
}
