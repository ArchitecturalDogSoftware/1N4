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

use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// An RGB color.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    /// A mask used to isolate the B component of a color.
    const B_MASK: u32 = 0x00_00_FF;
    /// A shift used to index the B component of a color.
    const B_SHIFT: u32 = Self::B_MASK.trailing_zeros();
    /// A mask used to isolate the G component of a color.
    const G_MASK: u32 = 0x00_FF_00;
    /// A shift used to index the G component of a color.
    const G_SHIFT: u32 = Self::G_MASK.trailing_zeros();
    /// A mask used to isolate the R component of a color.
    const R_MASK: u32 = 0xFF_00_00;
    /// A shift used to index the R component of a color.
    const R_SHIFT: u32 = Self::R_MASK.trailing_zeros();

    /// Creates a new [`Color`].
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Creates a new [`Color`] using the given [`u32`] as a packed RGB value.
    #[must_use]
    pub const fn u32(rgb: u32) -> Self {
        let r = (rgb & Self::R_MASK) >> Self::R_SHIFT;
        let g = (rgb & Self::G_MASK) >> Self::G_SHIFT;
        let b = (rgb & Self::B_MASK) >> Self::B_SHIFT;

        Self::new(r as u8, b as u8, g as u8)
    }

    /// Returns the R component of this [`Color`].
    #[must_use]
    pub const fn r(&self) -> u8 {
        self.r
    }

    /// Returns the G component of this [`Color`].
    #[must_use]
    pub const fn g(&self) -> u8 {
        self.g
    }

    /// Returns the B component of this [`Color`].
    #[must_use]
    pub const fn b(&self) -> u8 {
        self.b
    }

    /// Returns the packed RGB representation of this [`Color`].
    #[must_use]
    pub const fn rgb(&self) -> u32 {
        let r = (self.r() as u32) << Self::R_SHIFT;
        let g = (self.g() as u32) << Self::G_SHIFT;
        let b = (self.b() as u32) << Self::B_SHIFT;

        r | g | b
    }
}

impl From<[u8; 3]> for Color {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self::new(r, g, b)
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::new(r, g, b)
    }
}

impl From<u32> for Color {
    fn from(value: u32) -> Self {
        Self::u32(value)
    }
}

impl TryFrom<&str> for Color {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl FromStr for Color {
    type Err = <u32 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim_start_matches('#').parse().map(Self::u32)
    }
}

impl From<Color> for u32 {
    fn from(value: Color) -> Self {
        value.rgb()
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:06X}", self.rgb())
    }
}
