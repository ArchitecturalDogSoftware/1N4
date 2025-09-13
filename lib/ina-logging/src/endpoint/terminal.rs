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

use owo_colors::Stream;
use tokio::io::{AsyncWriteExt, Stderr};

use super::Endpoint;
use crate::Result;
use crate::entry::Entry;
use crate::settings::Settings;

/// A logger endpoint for the terminal.
#[derive(Debug, Default)]
pub struct TerminalEndpoint {
    /// The standard error stream.
    stderr: Option<Stderr>,
}

impl TerminalEndpoint {
    /// Creates a new [`TerminalEndpoint`].
    #[must_use]
    pub const fn new() -> Self {
        Self { stderr: None }
    }
}

#[async_trait::async_trait]
impl Endpoint for TerminalEndpoint {
    fn name(&self) -> &'static str {
        "terminal"
    }

    async fn setup(&mut self, _: &Settings) -> Result<()> {
        self.stderr = Some(tokio::io::stderr());

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        if let Some(mut stderr) = self.stderr.take() {
            stderr.shutdown().await?;
        }

        Ok(())
    }

    async fn write(&mut self, entry: &Entry<'static>) -> Result<()> {
        let content = entry.display(Some(Stream::Stderr)).to_string() + "\n";
        let Some(ref mut stderr) = self.stderr else {
            return Err(self.invalid_state());
        };

        stderr.write_all(content.as_bytes()).await.map_err(Into::into)
    }
}
