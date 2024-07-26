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
use ina_logging::info;

/// The application's command-line arguments.
#[derive(Clone, Debug, PartialEq, Eq, Parser)]
pub struct Arguments {
    /// The localization thread's settings.
    #[command(flatten)]
    pub lang_settings: ina_localization::Settings,
    /// The logging thread's settings.
    #[command(flatten)]
    pub log_settings: ina_logging::Settings,
}

/// The application's main entrypoint.
///
/// # Errors
///
/// This function will return an error if the program's execution fails.
pub fn main() -> Result<()> {
    let arguments = Arguments::parse();

    ina_logging::thread::blocking_start(arguments.log_settings.clone())?;

    info!("initialized logging thread")?;

    #[cfg(feature = "dotenv")]
    {
        dotenvy::dotenv()?;

        info!("loaded environment variables")?;
    }

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

    runtime.block_on(self::async_main(arguments))?;

    info!("closing logging thread")?;

    ina_logging::thread::blocking_close();

    Ok(())
}

/// The application's main function.
///
/// # Errors
///
/// This function will return an error if the program's execution fails.
pub async fn async_main(arguments: Arguments) -> Result<()> {
    info!(async "entering asynchronous runtime").await?;

    ina_localization::thread::start(arguments.lang_settings).await?;

    info!(async "initialized localization thread").await?;

    // TODO: Main program execution.

    info!(async "exiting asynchronous runtime").await?;

    ina_localization::thread::close().await;

    info!(async "closed localization thread").await?;

    Ok(())
}
