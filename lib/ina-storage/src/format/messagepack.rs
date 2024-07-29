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

use super::{DataDecode, DataEncode, DataFormat};

/// The Messagepack data format.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Messagepack;

impl DataFormat for Messagepack {
    #[inline]
    fn extension(&self) -> impl AsRef<OsStr> {
        "pack"
    }
}

impl DataEncode for Messagepack {
    type Error = rmp_serde::encode::Error;

    #[inline]
    fn encode<T: Serialize>(&self, value: &T) -> Result<Arc<[u8]>, Self::Error> {
        rmp_serde::to_vec_named(value).map(Into::into)
    }
}

impl DataDecode for Messagepack {
    type Error = rmp_serde::decode::Error;

    #[inline]
    fn decode<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> Result<T, Self::Error> {
        rmp_serde::from_slice(bytes)
    }
}
