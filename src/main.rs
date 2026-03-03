// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024 Jaxydog
// Copyright © 2025 RemasteredArch
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

use std::fs::File;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use ina_macro::optional;
use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;
use tracing::{debug, info};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::fmt::writer::Tee;

use crate::client::Instance;

/// The bot's client implementation.
pub mod client;
/// The bot's commands and command registry.
pub mod command;
/// Provides commonly used definitions.
pub mod utility;

/// The application's command-line arguments.
#[non_exhaustive]
#[optional(
    keep_annotations = [non_exhaustive],
    apply_derives = [Clone, Debug, Hash, PartialEq, Eq],
)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Parser, Serialize)]
#[command(about, version)]
pub struct Arguments {
    /// The bot's settings.
    #[option(flatten)]
    #[serde(rename = "client")]
    pub bot_settings: crate::client::settings::Settings,
    /// The storage instance's settings.
    #[option(flatten)]
    #[serde(rename = "storage")]
    pub data_settings: ina_storage::settings::Settings,
    /// The localization thread's settings.
    #[option(flatten)]
    #[serde(rename = "localizer")]
    pub lang_settings: ina_localizing::settings::Settings,
}

/// The application's main entry-point.
///
/// # Errors
///
/// This function will return an error if the program's execution fails.
pub fn main() -> Result<ExitCode> {
    // Safety:
    // The requirement here is that this is called in a single-threaded context, or while no other threads are reading
    // from the environment while this is being set.
    //
    // In this case, this is the very first function being called, which means that no other threads are actively
    // accessing any environment variables.
    //
    // This block is BLUE. This represents the BLUE that MIKU has taken on for us all. Da ba dee. Da ba di.
    #[cfg(debug_assertions)]
    #[expect(unsafe_code, reason = "setting environment variables at run-time requires the usage of unsafe")]
    unsafe {
        // We only want to capture backtraces on debug builds, as this can have a large performance impact.
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    let arguments = get_config();

    self::initialize_logger(&arguments)?;

    let timeout = Duration::from_secs(arguments.bot_settings.shutdown_timeout.get());
    ina_threading::blocking_set_runtime_timeout(timeout);

    info!("initialized logging subscriber");

    #[cfg(feature = "dotenv")]
    {
        let path = dotenvy::dotenv()?;
        info!(?path, "loaded environment variables from file");
    }

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    info!(id = %runtime.handle().id(), workers = runtime.metrics().num_workers(), "spawned asynchronous runtime");

    let code = runtime.block_on(self::async_main(arguments))?;
    info!(?code, "exited asynchronous runtime");

    runtime.shutdown_timeout(timeout);
    debug!("exiting program");

    Ok(code)
}

/// The application's main function.
///
/// # Errors
///
/// This function will return an error if the program's execution fails.
#[tracing::instrument(level = "trace", name = "rt_m", skip_all)]
pub async fn async_main(arguments: Arguments) -> Result<ExitCode> {
    let runtime = tokio::runtime::Handle::current();
    info!(id = %runtime.id(), workers = runtime.metrics().num_workers(), "entered asynchronous runtime");

    ina_localizing::thread::start(arguments.lang_settings).await?;
    info!("initialized localization thread");

    let count = ina_localizing::thread::load(None::<[_; 0]>).await?;
    info!(count, "loaded localizer locales");

    ina_storage::format::encryption::set_password_resolver(|| {
        crate::utility::secret::encryption_key().map(|v| v.to_string()).ok()
    });
    ina_storage::thread::start(arguments.data_settings).await?;
    info!("initialized storage thread");

    let instance = Instance::new(arguments.bot_settings).await?;
    info!("initialized client instance");

    tokio::pin! {
        let process = instance.run();
        let terminate = tokio::signal::ctrl_c();
    }

    info!("starting client process");

    let code = tokio::select! {
        // Exit code of 130 for ^C is standard; 128 (to mark a signal) + 2 (the code for the ^C interrupt).
        _ = terminate => {
            info!("received termination signal");

            ExitCode::from(130)
        }
        result = process => match result {
            Ok(()) => {
                info!("client process shut down naturally");

                ExitCode::SUCCESS
            }
            Err(error) => {
                tracing::error!(%error, "unhandled error encountered");

                ExitCode::FAILURE
            }
        },
    };

    ina_storage::thread::close().await;
    info!("closed storage thread");

    ina_localizing::thread::close().await;
    info!("closed localization thread");

    Ok(code)
}

/// Resolve command-line arguments.
///
/// This is distinct from just running [`OptionalArguments::fill_defaults`] on [`OptionalArguments::parse`] because it
/// applies extra changes on top.
fn get_config() -> Arguments {
    let mut args = OptionalArguments::parse().fill_defaults();

    if args.bot_settings.quiet {
        args.bot_settings.disable_file_logging = true;
        args.bot_settings.disable_console_logging = true;
    }
    args.bot_settings.quiet = args.bot_settings.disable_file_logging && args.bot_settings.disable_console_logging;

    args
}

/// Initializes the logging subscriber.
///
/// # Errors
///
/// This function will return an error if the log directory or output file could not be created.
fn initialize_logger(arguments: &Arguments) -> Result<()> {
    const DEFAULT_FILTER: LevelFilter = if cfg!(debug_assertions) { LevelFilter::DEBUG } else { LevelFilter::INFO };
    const FILE_NAME_FORMAT: &[FormatItem<'static>] = format_description!(
        version = 2,
        "[year repr:last_two][month padding:zero repr:numerical][day padding:zero]-[hour padding:zero][minute \
         padding:zero][second padding:zero]-[subsecond digits:6]"
    );

    let filter = EnvFilter::builder()
        .with_env_var("INA_LOG_LEVEL")
        .with_default_directive(DEFAULT_FILTER.into())
        .from_env_lossy();
    let subscriber = tracing_subscriber::fmt()
        .with_thread_names(true)
        .with_env_filter(filter)
        .with_ansi(arguments.bot_settings.color.is_supported_on(supports_color::Stream::Stdout));

    if !arguments.bot_settings.disable_file_logging {
        let current_time = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());

        let Ok(file_name) = current_time.format(FILE_NAME_FORMAT) else {
            unreachable!("the program will not compile if this format description is invalid");
        };

        let file_path = arguments.bot_settings.log_directory.join(file_name).with_extension("log");

        std::fs::create_dir_all(&arguments.bot_settings.log_directory)?;

        let file = File::options().create(true).append(true).open(file_path)?;

        if arguments.bot_settings.disable_console_logging {
            subscriber.with_writer(file).init();
        } else {
            subscriber.with_writer(Tee::new(std::io::stdout, file)).init();
        }
    } else if !arguments.bot_settings.disable_console_logging {
        subscriber.with_writer(std::io::stdout).init();
    }

    Ok(())
}
