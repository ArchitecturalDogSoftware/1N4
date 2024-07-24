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

//! Your resident M41D Unit, here to help with your server.

use anyhow::Result;
use clap::Parser;
use ina_logging::{debug, info, Settings};

/// The application's command-line arguments.
#[derive(Clone, Debug, PartialEq, Eq, Parser)]
pub struct Arguments {
    /// The logging thread's settings.
    #[command(flatten)]
    pub log_settings: Settings,
}

/// The application's main entrypoint.
///
/// # Errors
///
/// This function will return an error if the program's execution fails.
pub fn main() -> Result<()> {
    let arguments = Arguments::parse();

    ina_logging::thread::start(arguments.log_settings)?;

    debug!("does this work in release?")?;
    info!("it sure does!")?;

    ina_logging::thread::close();

    Ok(())
}
