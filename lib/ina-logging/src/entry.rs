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

use time::OffsetDateTime;

/// A log entry's timestamp.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timestamp {
    /// The entry's creation time.
    pub time: OffsetDateTime,
}

impl Timestamp {
    /// Creates a new [`Timestamp`].
    #[must_use]
    pub fn new() -> Self {
        Self { time: OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc()) }
    }

    /// Returns a display implementation for this [`Timestamp`].
    #[must_use]
    pub const fn display(
        &self,
        #[cfg(feature = "terminal")] stream: Option<owo_colors::Stream>,
    ) -> self::display::TimestampDisplay {
        self::display::TimestampDisplay {
            timestamp: self,
            #[cfg(feature = "terminal")]
            stream,
        }
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::new()
    }
}

/// A log entry's level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level<'lv> {
    /// The log level's name.
    pub name: &'lv str,
    /// The log level's color.
    #[cfg(feature = "terminal")]
    pub color: owo_colors::AnsiColors,
    /// Whether logs of this type are considered errors.
    pub error: bool,
}

impl Level<'_> {
    /// The debug log level.
    #[cfg(debug_assertions)]
    pub const DEBUG: Self = Self {
        name: "debug",
        #[cfg(feature = "terminal")]
        color: owo_colors::AnsiColors::BrightMagenta,
        error: false,
    };
    /// The error log level.
    pub const ERROR: Self = Self {
        name: "error",
        #[cfg(feature = "terminal")]
        color: owo_colors::AnsiColors::BrightRed,
        error: true,
    };
    /// The info log level.
    pub const INFO: Self = Self {
        name: "info",
        #[cfg(feature = "terminal")]
        color: owo_colors::AnsiColors::BrightBlue,
        error: false,
    };
    /// The warning log level.
    pub const WARN: Self = Self {
        name: "warn",
        #[cfg(feature = "terminal")]
        color: owo_colors::AnsiColors::BrightYellow,
        error: true,
    };

    /// Returns a display implementation for this [`Level`].
    #[must_use]
    pub const fn display(
        &self,
        #[cfg(feature = "terminal")] stream: Option<owo_colors::Stream>,
    ) -> self::display::LevelDisplay {
        self::display::LevelDisplay {
            level: self,
            #[cfg(feature = "terminal")]
            stream,
        }
    }
}

/// A log entry.
#[derive(Clone, Debug)]
pub struct Entry<'lv> {
    /// The entry's timestamp.
    pub timestamp: Timestamp,
    /// The entry's level.
    pub level: Level<'lv>,
    /// The entry's content.
    pub content: Arc<str>,
}

impl<'lv> Entry<'lv> {
    /// Creates a new [`Entry`].
    #[must_use]
    pub fn new(level: Level<'lv>, content: Arc<str>) -> Self {
        Self { timestamp: Timestamp::new(), level, content }
    }

    /// Returns a display implementation for this [`Entry`].
    #[must_use]
    pub const fn display(
        &self,
        #[cfg(feature = "terminal")] stream: Option<owo_colors::Stream>,
    ) -> self::display::EntryDisplay {
        self::display::EntryDisplay {
            entry: self,
            #[cfg(feature = "terminal")]
            stream,
        }
    }
}

/// Provides various display interfaces for entries.
pub mod display {
    use std::fmt::Display;

    use time::format_description::well_known::Iso8601;

    use super::{Entry, Level, Timestamp};

    /// Displays a time stamp.
    #[cfg_attr(not(feature = "terminal"), repr(transparent))]
    #[derive(Clone, Copy, Debug)]
    pub struct TimestampDisplay<'r> {
        /// The displayed timestamp.
        pub(super) timestamp: &'r Timestamp,
        /// The output stream, if outputting to the terminal.
        #[cfg(feature = "terminal")]
        pub(super) stream: Option<owo_colors::Stream>,
    }

    #[cfg(feature = "terminal")]
    impl Display for TimestampDisplay<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            use owo_colors::OwoColorize;

            let time = self.timestamp.time.format(&Iso8601::DEFAULT).unwrap_or_else(|_| unreachable!());

            if let Some(stream) = self.stream {
                write!(f, "{}", format_args!("[{time}]").if_supports_color(stream, |v| v.dimmed()))
            } else {
                write!(f, "[{time}]")
            }
        }
    }

    #[cfg(not(feature = "terminal"))]
    impl<'r> Display for TimestampDisplay<'r> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "[{}]", self.timestamp.time.format(&Iso8601::DEFAULT).unwrap_or_else(|_| unreachable!()))
        }
    }

    /// Displays a log level.
    #[cfg_attr(not(feature = "terminal"), repr(transparent))]
    #[derive(Clone, Copy, Debug)]
    pub struct LevelDisplay<'lv: 'r, 'r> {
        /// The displayed log level.
        pub(super) level: &'r Level<'lv>,
        /// The output stream, if outputting to the terminal.
        #[cfg(feature = "terminal")]
        pub(super) stream: Option<owo_colors::Stream>,
    }

    #[cfg(feature = "terminal")]
    impl<'lv: 'r, 'r> Display for LevelDisplay<'lv, 'r> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            use owo_colors::OwoColorize;

            if let Some(stream) = self.stream {
                let Self { level: Level { name, color, .. }, .. } = self;

                write!(f, "{}", format_args!("({name})").if_supports_color(stream, |v| v.color(*color)))
            } else {
                write!(f, "({})", self.level.name)
            }
        }
    }

    #[cfg(not(feature = "terminal"))]
    impl<'lv: 'r, 'r> Display for LevelDisplay<'lv, 'r> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "({})", self.level.name)
        }
    }

    /// Displays an entry.
    #[cfg_attr(not(feature = "terminal"), repr(transparent))]
    #[derive(Clone, Copy, Debug)]
    pub struct EntryDisplay<'lv: 'r, 'r> {
        /// The displayed entry.
        pub(super) entry: &'r Entry<'lv>,
        /// The output stream, if outputting to the terminal.
        #[cfg(feature = "terminal")]
        pub(super) stream: Option<owo_colors::Stream>,
    }

    impl<'lv: 'r, 'r> Display for EntryDisplay<'lv, 'r> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let timestamp = self.entry.timestamp.display(
                #[cfg(feature = "terminal")]
                self.stream,
            );
            let level = self.entry.level.display(
                #[cfg(feature = "terminal")]
                self.stream,
            );

            write!(f, "{timestamp} {level} {}", self.entry.content)
        }
    }
}
