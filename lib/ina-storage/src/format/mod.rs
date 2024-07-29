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
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[cfg(feature = "format-compression")]
pub use self::compression::Compress;
#[cfg(feature = "format-json")]
pub use self::json::Json;
#[cfg(feature = "format-messagepack")]
pub use self::messagepack::Messagepack;
#[cfg(feature = "format-postcard")]
pub use self::postcard::Postcard;
#[cfg(feature = "format-xmachina")]
pub use self::xmachina::XMachina;

/// The compression format.
#[cfg(feature = "format-compression")]
pub mod compression;
/// The JSON format.
#[cfg(feature = "format-json")]
pub mod json;
/// The Messagepack format.
#[cfg(feature = "format-messagepack")]
pub mod messagepack;
/// The Postcard format.
#[cfg(feature = "format-postcard")]
pub mod postcard;
/// The xmachina format.
#[cfg(feature = "format-xmachina")]
pub mod xmachina;

/// A value that encodes and decodes generic data.
pub trait DataFormat: DataDecode + DataEncode {
    /// Returns the file extension for this format.
    fn extension(&self) -> impl AsRef<OsStr>;
}

/// A value that encodes generic data.
pub trait DataEncode {
    /// The error that can be returned during encoding.
    type Error: std::fmt::Debug + std::error::Error + Send + Sync + 'static;

    /// Encodes the given value into a byte array.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value cannot be encoded.
    fn encode<T: Serialize>(&self, value: &T) -> Result<Arc<[u8]>, Self::Error>;
}

/// A value that decodes generic data.
pub trait DataDecode {
    /// The error that can be returned during decoding.
    type Error: std::fmt::Debug + std::error::Error + Send + Sync + 'static;

    /// Decodes the given byte array into a value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value cannot be decoded.
    fn decode<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> Result<T, Self::Error>;
}
