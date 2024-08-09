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

use owo_colors::Stream;
use tokio::io::{AsyncWriteExt, Stderr, Stdout};

use super::Endpoint;
use crate::entry::Entry;
use crate::settings::Settings;
use crate::Result;

/// A logger endpoint for the terminal.
#[derive(Debug, Default)]
pub struct TerminalEndpoint {
    /// The standard output stream.
    stdout: Option<Stdout>,
    /// The standard error stream.
    stderr: Option<Stderr>,
}

impl TerminalEndpoint {
    /// Creates a new [`TerminalEndpoint`].
    #[must_use]
    pub const fn new() -> Self {
        Self { stdout: None, stderr: None }
    }
}

#[async_trait::async_trait]
impl Endpoint for TerminalEndpoint {
    fn name(&self) -> &'static str {
        "terminal"
    }

    async fn setup(&mut self, _: &Settings) -> Result<()> {
        self.stdout = Some(tokio::io::stdout());
        self.stderr = Some(tokio::io::stderr());

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        drop(self.stdout.take());
        drop(self.stderr.take());

        Ok(())
    }

    async fn write(&mut self, entry: Arc<Entry<'static>>) -> Result<()> {
        let stream = if entry.level.error { Stream::Stderr } else { Stream::Stdout };
        let content = entry.display(Some(stream)).to_string() + "\n";

        if entry.level.error {
            let Some(ref mut stdout) = self.stdout else {
                return Err(self.invalid_state());
            };

            stdout.write_all(content.as_bytes()).await
        } else {
            let Some(ref mut stderr) = self.stderr else {
                return Err(self.invalid_state());
            };

            stderr.write_all(content.as_bytes()).await
        }
        .map_err(Into::into)
    }
}
