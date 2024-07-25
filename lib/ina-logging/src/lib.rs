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

//! Provides logging solutions for 1N4.

use std::convert::Infallible;
use std::fmt::Display;
use std::fs::File;
use std::io::{StderrLock, StdoutLock, Write};
use std::num::{NonZeroU64, NonZeroUsize};
use std::path::Path;
use std::time::Duration;

use clap::{Args, ColorChoice};
use owo_colors::{AnsiColors, OwoColorize, Stream};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Iso8601;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use tokio::sync::mpsc::error::SendError;

/// Contains the logger's thread implementation.
pub mod thread;

/// A result alias with a defaulted error type.
pub type Result<T, S = Infallible> = std::result::Result<T, Error<S>>;

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

/// The logger's settings.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Args, Serialize, Deserialize)]
pub struct Settings {
    /// Sets the logger's color preferences.
    #[arg(short = 'c', long = "color", default_value = "auto")]
    #[serde(with = "color_choice")]
    pub color: ColorChoice,

    /// Disables terminal output.
    #[arg(short = 'q', long = "quiet")]
    #[serde(rename = "quiet")]
    pub no_term_output: bool,
    /// Disables file output.
    #[arg(short = 'e', long = "ephemeral")]
    #[serde(rename = "ephemeral")]
    pub no_file_output: bool,

    /// The directory within which to output log files.
    #[arg(long = "log-directory", default_value = "./log/")]
    #[serde(rename = "directory")]
    pub file_directory: Box<Path>,

    /// The logger's output queue capacity. If set to '1', no buffering will be done.
    #[arg(long = "log-queue-capacity", default_value = "8")]
    #[serde(rename = "queue-capacity")]
    pub queue_capacity: NonZeroUsize,
    /// The logger's output queue timeout in milliseconds.
    #[arg(long = "log-queue-timeout", default_value = "20")]
    #[serde(rename = "queue-timeout")]
    pub queue_timeout: NonZeroU64,
}

/// A logger with a buffered output.
#[derive(Debug)]
pub struct Logger<'lg> {
    /// The logger's settings.
    settings: Settings,
    /// The logger's queue.
    queue: Vec<Entry<'lg>>,

    /// A write lock for stdout.
    out: Option<StdoutLock<'lg>>,
    /// A write lock for stderr.
    err: Option<StderrLock<'lg>>,
    /// A write lock for the output file.
    file: Option<File>,
}

impl<'lg> Logger<'lg> {
    /// The time formatter used to create log file names.
    const FILE_NAME_FORMAT: &'static [FormatItem<'static>] = format_description!(
        version = 2,
        "[year repr:last_two][month padding:zero repr:numerical][day padding:zero]-[hour padding:zero][minute \
         padding:zero][second padding:zero]-[subsecond digits:6]"
    );

    /// Creates a new [`Logger`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the logger failed to create its file.
    pub fn new(settings: Settings) -> Result<Self> {
        let queue = Vec::with_capacity(settings.queue_capacity.get());
        let out = (!settings.no_term_output).then(|| std::io::stdout().lock());
        let err = (!settings.no_term_output).then(|| std::io::stderr().lock());
        let file = (!settings.no_file_output).then(|| Self::new_file(&settings.file_directory)).transpose()?;

        Ok(Self { settings, queue, out, err, file })
    }

    /// Creates a new log file within the given directory.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file could not be created.
    pub(crate) fn new_file(directory: &Path) -> Result<File> {
        let time = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        let name = time.format(Self::FILE_NAME_FORMAT).unwrap_or_else(|_| unreachable!());
        let path = directory.join(name).with_extension("log");

        std::fs::create_dir_all(directory)?;

        File::options().create(true).append(true).open(path).map_err(Into::into)
    }

    /// Returns whether this [`Logger`] is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        !self.settings.no_term_output || !self.settings.no_file_output
    }

    /// Returns whether this [`Logger`] is disabled.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.settings.no_term_output && self.settings.no_file_output
    }

    /// Returns whether this [`Logger`] has entry buffering.
    #[must_use]
    pub const fn is_buffered(&self) -> bool {
        self.capacity().get() == 1
    }

    /// Returns the queue capacity of this [`Logger`].
    #[must_use]
    pub const fn capacity(&self) -> NonZeroUsize {
        self.settings.queue_capacity
    }

    /// Returns the queue timeout of this [`Logger`].
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        Duration::from_millis(self.settings.queue_timeout.get())
    }

    /// Returns the number of entries within the inner queue of this [`Logger`].
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Returns whether the inner queue of this [`Logger`] is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns whether the inner queue of this [`Logger`] is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity().get()
    }

    /// Queues a log to be output during the next flush.
    ///
    /// If this logger's buffer capacity is 1, this will flush immediately.
    ///
    /// # Errors
    ///
    /// This function will return an error if the logger could not be flushed.
    pub fn queue(&mut self, entry: Entry<'lg>) -> Result<()> {
        if self.is_disabled() {
            return Ok(());
        }

        self.queue.push(entry);

        if self.is_full() { self.flush() } else { Ok(()) }
    }

    /// Flushes the inner queue of this [`Logger`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the queue could not be flushed.
    pub fn flush(&mut self) -> Result<()> {
        /// Writes the given display into a possibly [`None`] writer.
        #[inline]
        fn writeln(writer: Option<&mut impl Write>, display: impl Display) -> Result<()> {
            writer.map(|w| writeln!(w, "{display}")).transpose().map(|_| ()).map_err(Into::into)
        }

        for entry in self.queue.drain(..) {
            if !self.settings.no_term_output {
                if entry.level.error {
                    writeln(self.err.as_mut(), entry.as_display(Some(Stream::Stderr)))?;
                } else {
                    writeln(self.out.as_mut(), entry.as_display(Some(Stream::Stdout)))?;
                }
            }

            if !self.settings.no_file_output {
                writeln(self.file.as_mut(), entry.as_display(None))?;
            }
        }

        Ok(())
    }
}

/// A log level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level<'lv> {
    /// The log level's name.
    pub name: &'lv str,
    /// The log level's color.
    pub color: AnsiColors,
    /// Whether this level is considered to be an error.
    pub error: bool,
}

impl<'lv> Level<'lv> {
    /// The debug log level.
    #[cfg(debug_assertions)]
    pub const DEBUG: Self = Self::new("debug", AnsiColors::BrightMagenta, false);
    /// The error log level.
    pub const ERROR: Self = Self::new("error", AnsiColors::BrightRed, true);
    /// The informational log level.
    pub const INFO: Self = Self::new("info", AnsiColors::BrightBlue, false);
    /// The warning log level.
    pub const WARN: Self = Self::new("warn", AnsiColors::BrightYellow, true);

    /// Creates a new [`Level`].
    #[must_use]
    pub const fn new(name: &'lv str, color: AnsiColors, error: bool) -> Self {
        Self { name, color, error }
    }

    /// Returns a display implementation for this level.
    #[must_use]
    pub const fn as_display(&'lv self, stream: Option<Stream>) -> impl Display + 'lv {
        #[derive(Clone, Copy, Debug)]
        struct LevelDisplay<'lv>(&'lv Level<'lv>, Option<Stream>);

        impl<'lv> Display for LevelDisplay<'lv> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let Self(Level { name, color, .. }, stream) = *self;

                if let Some(stream) = stream {
                    write!(f, "{}", format_args!("({name})").if_supports_color(stream, |v| v.color(*color)))
                } else {
                    write!(f, "({name})")
                }
            }
        }

        LevelDisplay(self, stream)
    }
}

/// A log entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry<'lv> {
    /// The entry's creation time.
    pub time: OffsetDateTime,
    /// The entry's log level.
    pub level: Level<'lv>,
    /// The entry's text.
    pub text: Box<str>,
}

impl<'lv> Entry<'lv> {
    /// Creates a new [`Entry`].
    pub fn new(level: Level<'lv>, text: impl AsRef<str>) -> Self {
        let time = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());

        Self { time, level, text: Box::from(text.as_ref()) }
    }

    /// Returns a display implementation for this entry.
    #[must_use]
    pub const fn as_display(&'lv self, stream: Option<Stream>) -> impl Display + 'lv {
        #[derive(Clone, Copy, Debug)]
        struct EntryDisplay<'lv>(&'lv Entry<'lv>, Option<Stream>);

        impl<'lv> Display for EntryDisplay<'lv> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let Self(Entry { time, level, text }, stream) = *self;
                let time = time.format(&Iso8601::DEFAULT).unwrap_or_else(|_| unreachable!());
                let level = level.as_display(stream);

                if let Some(stream) = stream {
                    write!(f, "{} {level} {text}", format_args!("[{time}]").if_supports_color(stream, |v| v.dimmed()))
                } else {
                    write!(f, "[{time}] {level} {text}")
                }
            }
        }

        EntryDisplay(self, stream)
    }
}

mod color_choice {
    use clap::ColorChoice;
    use serde::de::{Unexpected, Visitor};
    use serde::{Deserializer, Serializer};

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S>(value: &ColorChoice, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match value {
            ColorChoice::Auto => "auto",
            ColorChoice::Always => "always",
            ColorChoice::Never => "never",
        })
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ColorChoice, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ChoiceVisitor;

        impl Visitor<'_> for ChoiceVisitor {
            type Value = ColorChoice;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a color choice")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "auto" => Ok(ColorChoice::Auto),
                    "always" => Ok(ColorChoice::Always),
                    "never" => Ok(ColorChoice::Never),
                    _ => Err(E::invalid_value(Unexpected::Str(v), &self)),
                }
            }
        }

        deserializer.deserialize_str(ChoiceVisitor)
    }
}
