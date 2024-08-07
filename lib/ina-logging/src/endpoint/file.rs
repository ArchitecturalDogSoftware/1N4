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

use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use super::Endpoint;
use crate::entry::Entry;
use crate::settings::Settings;
use crate::Result;

/// The time formatter used to create log file names.
const FILE_NAME_FORMAT: &[FormatItem<'static>] = format_description!(
    version = 2,
    "[year repr:last_two][month padding:zero repr:numerical][day padding:zero]-[hour padding:zero][minute \
     padding:zero][second padding:zero]-[subsecond digits:6]"
);

/// A logger endpoint for a file.
#[derive(Debug, Default)]
pub struct FileEndpoint {
    /// The file handle.
    handle: Option<File>,
}

impl FileEndpoint {
    /// Creates a new [`FileEndpoint`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self { handle: None }
    }
}

#[async_trait::async_trait]
impl Endpoint for FileEndpoint {
    #[inline]
    fn name(&self) -> &'static str {
        "file"
    }

    async fn setup(&mut self, settings: &Settings) -> Result<()> {
        let time = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        let name = time.format(FILE_NAME_FORMAT).unwrap_or_else(|_| unreachable!());
        let path = settings.directory.join(name).with_extension("log");

        tokio::fs::create_dir_all(&settings.directory).await?;

        self.handle = Some(File::options().create(true).append(true).open(path).await?);

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        drop(self.handle.take());

        Ok(())
    }

    async fn write(&mut self, entry: Arc<Entry<'static>>) -> Result<()> {
        let content = entry.display(None).to_string() + "\n";
        let Some(ref mut handle) = self.handle else {
            return Err(self.invalid_state());
        };

        handle.write_all(content.as_bytes()).await.map_err(Into::into)
    }
}
