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

use std::fmt::{Display, LowerHex, UpperHex};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// An error returned when parsing a color from a string.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// An RGB component was missing from the string.
    #[error("the given string is missing at least one rgb component")]
    MissingRgbComponent,
    /// An HSL component was missing from the string.
    #[error("the given string is missing at least one hsl component")]
    MissingHslComponent,
    /// A value in the source string was unexpected.
    #[error("the given string appears to be invalid: '{0}'")]
    UnexpectedValue(Box<str>),
    /// An error during u8 parsing.
    #[error(transparent)]
    ParseU8(<u8 as FromStr>::Err),
    /// An error during u32 parsing.
    #[error(transparent)]
    ParseU32(<u32 as FromStr>::Err),
    /// An error during f64 parsing.
    #[error(transparent)]
    ParseF64(<f64 as FromStr>::Err),
}

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
    pub const fn from_u32(rgb: u32) -> Self {
        let r = (rgb & Self::R_MASK) >> Self::R_SHIFT;
        let g = (rgb & Self::G_MASK) >> Self::G_SHIFT;
        let b = (rgb & Self::B_MASK) >> Self::B_SHIFT;

        Self::new(r as u8, g as u8, b as u8)
    }

    /// Creates a new [`Color`] using the given scaled components.
    ///
    /// Each value is expected to be between 0 and 1, and will be clamped if it exits that threshold.
    #[expect(clippy::cast_sign_loss, reason = "we're clamping the values to always be positive")]
    #[expect(clippy::cast_possible_truncation, reason = "the product will always be at most 255")]
    #[must_use]
    pub const fn from_scaled(r: f64, g: f64, b: f64) -> Self {
        let r = (r.clamp(0.0, 1.0) * 255.0) as u8;
        let g = (g.clamp(0.0, 1.0) * 255.0) as u8;
        let b = (b.clamp(0.0, 1.0) * 255.0) as u8;

        Self::new(r, g, b)
    }

    /// Creates a new [`Color`] using the given HSL values.
    ///
    /// Hue should be within the range [0, 360), saturation should be within the range [0, 1], and lightness should be
    /// within [0, 1].
    ///
    /// Adapted from <https://en.wikipedia.org/wiki/HSL_and_HSV#HSL_to_RGB>.
    #[must_use]
    pub const fn from_hsl(hue: f64, saturation: f64, lightness: f64) -> Self {
        let chroma = (1.0 - ((2.0 * lightness) - 1.0).abs()) * saturation;
        let hue_prime = hue / 60.0;
        let x = chroma * (1.0 - ((hue_prime % 2.0) - 1.0).abs());

        let modifier = lightness - (chroma / 2.0);
        let (r1, g1, b1) = match hue_prime {
            0.0 .. 1.0 => (chroma, x, 0.0),
            1.0 .. 2.0 => (x, chroma, 0.0),
            2.0 .. 3.0 => (0.0, chroma, x),
            3.0 .. 4.0 => (0.0, x, chroma),
            4.0 .. 5.0 => (x, 0.0, chroma),
            5.0 .. 6.0 => (chroma, 0.0, x),
            _ => unreachable!(),
        };

        Self::from_scaled(r1 + modifier, g1 + modifier, b1 + modifier)
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

    /// Returns the R component of this [`Color`], scaled between 0-1.
    #[must_use]
    pub const fn r_scaled(&self) -> f64 {
        self.r as f64 / 255.0
    }

    /// Returns the G component of this [`Color`], scaled between 0-1.
    #[must_use]
    pub const fn g_scaled(&self) -> f64 {
        self.g as f64 / 255.0
    }

    /// Returns the B component of this [`Color`], scaled between 0-1.
    #[must_use]
    pub const fn b_scaled(&self) -> f64 {
        self.b as f64 / 255.0
    }

    /// Returns the packed RGB representation of this [`Color`].
    #[must_use]
    pub const fn rgb(&self) -> u32 {
        let r = (self.r() as u32) << Self::R_SHIFT;
        let g = (self.g() as u32) << Self::G_SHIFT;
        let b = (self.b() as u32) << Self::B_SHIFT;

        r | g | b
    }

    /// Returns the R, G, and B components of this [`Color`], all scaled between 0-1.
    #[must_use]
    pub const fn rgb_scaled(&self) -> (f64, f64, f64) {
        (self.r_scaled(), self.g_scaled(), self.b_scaled())
    }

    /// Returns the color's HSL values.
    ///
    /// Hue should be within the range [0, 360), saturation should be within the range [0, 1], and lightness should be
    /// within [0, 1].
    ///
    /// Adapted from <https://en.wikipedia.org/wiki/HSL_and_HSV#General_approach>.
    #[must_use]
    pub const fn hsl(&self) -> (f64, f64, f64) {
        let r = self.r_scaled();
        let g = self.g_scaled();
        let b = self.b_scaled();

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let chroma = max - min;

        let hue = if chroma < f64::EPSILON {
            0.0
        } else if (max - r).abs() < f64::EPSILON {
            ((g - b) / chroma) % 6.0
        } else if (max - g).abs() < f64::EPSILON {
            ((b - r) / chroma) + 2.0
        } else if (max - b).abs() < f64::EPSILON {
            ((r - g) / chroma) + 4.0
        } else {
            unreachable!()
        };

        let lightness = (max + min) / 2.0;
        let saturation = match lightness {
            0.0 | 1.0 => 0.0,
            _ => chroma / (1.0 - ((2.0 * lightness) - 1.0).abs()),
        };

        (hue, saturation, lightness)
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
        Self::from_u32(value)
    }
}

impl TryFrom<&str> for Color {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl FromStr for Color {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(hex_string) = s.strip_prefix('#') {
            u32::from_str_radix(hex_string, 16).map(Self::from_u32).map_err(ParseError::ParseU32)
        } else if let Some(rgb_string) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
            let rgb_string = rgb_string.replace(' ', "");
            let mut iterator = rgb_string.split(',');

            let Some(r_string) = iterator.next() else { return Err(ParseError::MissingRgbComponent) };
            let Some(g_string) = iterator.next() else { return Err(ParseError::MissingRgbComponent) };
            let Some(b_string) = iterator.next() else { return Err(ParseError::MissingRgbComponent) };

            if iterator.count() != 0 {
                return Err(ParseError::UnexpectedValue(s.into()));
            }

            if r_string.contains('.') || g_string.contains('.') || b_string.contains('.') {
                let r = r_string.parse().map_err(ParseError::ParseF64)?;
                let g = g_string.parse().map_err(ParseError::ParseF64)?;
                let b = b_string.parse().map_err(ParseError::ParseF64)?;

                Ok(Self::from_scaled(r, g, b))
            } else {
                let r = r_string.parse().map_err(ParseError::ParseU8)?;
                let g = g_string.parse().map_err(ParseError::ParseU8)?;
                let b = b_string.parse().map_err(ParseError::ParseU8)?;

                Ok(Self::new(r, g, b))
            }
        } else if let Some(hsl_string) = s.strip_prefix("hsl(").and_then(|s| s.strip_suffix(')')) {
            let hsl_string = hsl_string.replace(' ', "");
            let mut iterator = hsl_string.split(',');

            let Some(h_string) = iterator.next() else { return Err(ParseError::MissingHslComponent) };
            let Some(s_string) = iterator.next() else { return Err(ParseError::MissingHslComponent) };
            let Some(l_string) = iterator.next() else { return Err(ParseError::MissingHslComponent) };

            if iterator.count() != 0 {
                return Err(ParseError::UnexpectedValue(s.into()));
            }

            let h = h_string.parse().map_err(ParseError::ParseF64)?;
            let s = s_string.parse().map_err(ParseError::ParseF64)?;
            let l = l_string.parse().map_err(ParseError::ParseF64)?;

            Ok(Self::from_hsl(h, s, l))
        } else {
            s.parse().map(Self::from_u32).map_err(ParseError::ParseU32)
        }
    }
}

impl From<Color> for u32 {
    fn from(value: Color) -> Self {
        value.rgb()
    }
}

impl From<Color> for [u8; 3] {
    fn from(value: Color) -> Self {
        [value.r(), value.g(), value.b()]
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r(), self.g(), self.b())
    }
}

impl LowerHex for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:06x}", self.rgb())
    }
}

impl UpperHex for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:06X}", self.rgb())
    }
}
