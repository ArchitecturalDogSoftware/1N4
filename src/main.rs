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

#![feature(is_none_or)]

use anyhow::Result;
use clap::Parser;
use client::Instance;
use ina_logging::endpoint::{FileEndpoint, TerminalEndpoint};
use ina_logging::{error, info};
use serde::{Deserialize, Serialize};

/// The bot's client implementation.
pub mod client;
/// The bot's commands and command registry.
pub mod command;
/// Provides commonly used definitions.
pub mod utility;

/// The application's command-line arguments.
#[derive(Clone, Debug, PartialEq, Eq, Parser, Serialize, Deserialize)]
pub struct Arguments {
    /// The bot's settings.
    #[command(flatten)]
    #[serde(rename = "client")]
    pub bot_settings: client::Settings,
    /// The storage instance's settings.
    #[command(flatten)]
    #[serde(rename = "storage")]
    pub data_settings: ina_storage::Settings,
    /// The localization thread's settings.
    #[command(flatten)]
    #[serde(rename = "localizer")]
    pub lang_settings: ina_localization::Settings,
    /// The logging thread's settings.
    #[command(flatten)]
    #[serde(rename = "logger")]
    pub log_settings: ina_logging::settings::Settings,
}

/// The application's main entrypoint.
///
/// # Errors
///
/// This function will return an error if the program's execution fails.
pub fn main() -> Result<()> {
    let arguments = Arguments::parse();

    ina_logging::thread::blocking_start(arguments.log_settings.clone())?;
    ina_logging::thread::blocking_endpoint(TerminalEndpoint::new())?;
    ina_logging::thread::blocking_endpoint(FileEndpoint::new())?;
    ina_logging::thread::blocking_setup()?;

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
    info!(async "entered asynchronous runtime").await?;

    ina_localization::thread::start(arguments.lang_settings).await?;

    info!(async "initialized localization thread").await?;

    let loaded_locales = ina_localization::thread::load(None).await?;

    info!(async "loaded {loaded_locales} localization locales").await?;

    ina_storage::initialize(arguments.data_settings).await;

    info!(async "initialized storage instance").await?;

    let instance = Instance::new(arguments.bot_settings).await?;

    info!(async "initialized client instance").await?;

    tokio::pin! {
        let process = instance.run();
        let terminate = tokio::signal::ctrl_c();
    }

    info!(async "starting client process").await?;

    #[allow(clippy::redundant_pub_crate)] // False-positive?
    tokio::select! {
        _ = terminate => info!(async "received termination signal").await,
        result = process => match result {
            Ok(()) => info!(async "stopping client process").await,
            Err(error) => error!(async "unhandled error encountered: {error}").await,
        },
    }?;

    ina_localization::thread::close().await;

    info!(async "closed localization thread").await?;

    info!(async "exiting asynchronous runtime").await.map_err(Into::into)
}
