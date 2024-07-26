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

use std::future::Future;
use std::num::NonZeroU32;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use api::Api;
use clap::Args;
use ina_logging::warn;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;
use tokio_stream::{StreamExt, StreamMap};
use twilight_gateway::{Config, ConfigBuilder, EventTypeFlags, Intents, Shard};
use twilight_http::Client;
use twilight_model::gateway::connection_info::BotConnectionInfo;
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::presence::{ActivityType, MinimalActivity, Status};

/// Provides an API structure to be passed between functions.
pub mod api;
/// Provides an API for handling events.
pub mod event;

/// The bot's gateway intentions.
pub const INTENTS: Intents = Intents::empty()
    .union(Intents::DIRECT_MESSAGES)
    .union(Intents::DIRECT_MESSAGE_REACTIONS)
    .union(Intents::GUILDS)
    .union(Intents::GUILD_EMOJIS_AND_STICKERS)
    .union(Intents::GUILD_MEMBERS)
    .union(Intents::GUILD_MESSAGES)
    .union(Intents::GUILD_SCHEDULED_EVENTS)
    .union(Intents::GUILD_MESSAGE_REACTIONS)
    .union(Intents::MESSAGE_CONTENT);

/// The bot's settings.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Args, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[group(id = "BotSettings")]
pub struct Settings {
    /// The location of the file that determines the bot's status.
    #[arg(long = "status-file", default_value = "./res/status.toml")]
    pub status_file: Box<Path>,
    /// The number of shards to spawn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[arg(short = 's', long = "shards")]
    pub shards: Option<NonZeroU32>,
    /// The interval at which to reshard in hours.
    #[arg(short = 'r', long = "reshard-interval")]
    pub reshard_interval: u64,
}

/// The bot's status definition schema.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusList {
    /// The bot's testing status definitions.
    pub testing: Box<[StatusDefinition]>,
    /// The bot's release status definitions.
    pub release: Box<[StatusDefinition]>,
}

/// A status definition.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusDefinition {
    /// The status.
    pub status: Status,
    /// The activity type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<ActivityType>,
    /// The activity text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Box<str>>,
    /// The activity link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<Box<str>>,
}

/// The bot's instance.
#[derive(Debug)]
pub struct Instance {
    /// The canonical API instance.
    api: Api,
    /// The bot instance's created shards.
    shards: Box<[Shard]>,
    /// The bot instance's settings.
    settings: Settings,
    /// The bot's configured status list.
    status: Option<StatusList>,
}

impl Instance {
    /// Creates a new [`Instance`].
    ///
    /// # Errors
    ///
    /// This function will return an error if an [`Instance`] cannot be created.
    pub async fn new(settings: Settings) -> Result<Self> {
        let discord_token = crate::utility::secret::discord_token()?;
        let client = Client::new(discord_token.to_string());
        let status = Self::new_status(&settings).await?;
        let config = Self::new_config(discord_token.to_string(), status.as_ref())?;
        let shards = Self::new_shards(&client, config, &settings).await?;

        Ok(Self { api: Api::new(client), shards, settings, status })
    }

    /// Creates a new [`StatusList`], returning [`None`] if a file could not be found.
    ///
    /// # Errors
    ///
    /// This function will return an error if the [`StatusList`] could not be deserialized.
    pub async fn new_status(settings: &Settings) -> Result<Option<StatusList>> {
        let path = &(*settings.status_file);

        if !tokio::fs::try_exists(path).await? {
            return Ok(None);
        }

        let data = tokio::fs::read_to_string(path).await?;

        Ok(Some(toml::from_str(&data)?))
    }

    /// Creates a new [`Config`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the [`Config`] could not be created.
    pub fn new_config(token: String, status: Option<&StatusList>) -> Result<Config> {
        let mut builder = ConfigBuilder::new(token, self::INTENTS);

        if let Some(status) = status {
            let list = if cfg!(debug_assertions) { &status.testing } else { &status.release };
            let index = thread_rng().gen_range(0 .. list.len());
            let payload = Self::get_status(&list[index])?;

            builder = builder.presence(payload);
        }

        Ok(builder.build())
    }

    /// Creates a new list of shards.
    ///
    /// # Errors
    ///
    /// This function will return an error if the shards could not be created.
    pub async fn new_shards(client: &Client, config: Config, settings: &Settings) -> Result<Box<[Shard]>> {
        Ok(if let Some(count) = settings.shards.map(NonZeroU32::get) {
            twilight_gateway::create_iterator(0 .. count, count, config, |_, b| b.build()).collect()
        } else {
            twilight_gateway::create_recommended(client, config, |_, b| b.build()).await?.collect()
        })
    }

    /// Builds the given status definition into a payload.
    ///
    /// # Errors
    ///
    /// This function will return an error if the payload fails to build.
    pub(crate) fn get_status(definition: &StatusDefinition) -> Result<UpdatePresencePayload> {
        let activity = match (definition.activity, definition.content.as_deref(), definition.link.as_deref()) {
            // Only the type is provided.
            (Some(kind), None, None) => MinimalActivity { kind, name: String::new(), url: None },
            // The type and text are provided.
            (Some(kind), Some(text), None) => MinimalActivity { kind, name: text.to_string(), url: None },
            // The type and link are provided.
            (Some(kind), None, Some(link)) => {
                MinimalActivity { kind, name: String::new(), url: Some(link.to_string()) }
            }
            // All content is provided.
            (Some(kind), Some(text), Some(link)) => {
                MinimalActivity { kind, name: text.to_string(), url: Some(link.to_string()) }
            }
            // Any invalid combinations default.
            _ => MinimalActivity { kind: ActivityType::Custom, name: String::new(), url: None },
        };

        UpdatePresencePayload::new(vec![activity.into()], false, None, definition.status).map_err(Into::into)
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
    pub(crate) fn get_shard_timeout(connection: &BotConnectionInfo) -> Duration {
        const DAY: Duration = Duration::from_secs(60 * 60 * 24);

        let timeout = Duration::from_millis(connection.session_start_limit.reset_after);
        let refills = connection.shards / connection.session_start_limit.remaining;
        let buckets = (connection.shards as f32) / f32::from(connection.session_start_limit.max_concurrency);
        let buckets = buckets.round() as u64;

        timeout * u32::from(refills > 0)
            + (1 .. refills).map(|_| DAY).sum::<Duration>()
            + Duration::from_secs(5 * buckets % u64::from(connection.session_start_limit.total))
    }

    /// Attempts to re-shard this [`Instance`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the instance could not be re-sharded.
    pub(crate) async fn try_reshard(
        client: &Client,
        settings: &Settings,
        status: Option<&StatusList>,
    ) -> Result<Box<[Shard]>> {
        let seconds = settings.reshard_interval.saturating_mul(60 * 60);

        tokio::time::sleep(Duration::from_secs(seconds)).await;

        let connection = client.gateway().authed().await?.model().await?;
        let discord_token = crate::utility::secret::discord_token()?.to_string();
        let config = Self::new_config(discord_token, status)?;
        let mut shards = Self::new_shards(client, config, settings).await?;

        let timeout = tokio::time::sleep(Self::get_shard_timeout(&connection));

        tokio::pin!(timeout);

        std::future::poll_fn(|cx| {
            let _ = timeout.as_mut().poll(cx);

            std::task::Poll::Ready(())
        })
        .await;

        // Attempt to identify to make the swap cleaner.
        let mut identified = vec![false; shards.len()].into_boxed_slice();
        let mut shard_stream = shards.iter_mut().map(|s| (s.id(), s)).collect::<StreamMap<_, _>>();

        loop {
            let identified_count = identified.iter().copied().map(usize::from).sum::<usize>();

            tokio::select! {
                // Exit early if we time out and at least 75% of the shards are identified.
                () = &mut timeout, if identified_count >= (identified.len() * 3) / 4 => break,
                Some((shard_id, result)) = shard_stream.next() => {
                    if let Err(error) = result {
                        warn!(async "failed to identify shard: {error}").await?;
                        continue;
                    }

                    #[allow(clippy::expect_used)]
                    let shard = shard_stream.values().find(|s| s.id() == shard_id).unwrap_or_else(|| unreachable!());
                    let is_identified = shard.state().is_identified();

                    identified[shard_id.number() as usize] = is_identified;
                }
            }
        }

        Ok(shards)
    }

    /// Runs the bot application.
    ///
    /// # Errors
    ///
    /// This function will return an error if the instance encounters an unhandled exception.
    pub async fn run(mut self) -> Result<()> {
        loop {
            let mut tasks = JoinSet::new();

            for shard in self.shards {
                tasks.spawn(Self::run_shard(self.api.clone(), shard));
            }

            let shards = Self::try_reshard(&self.api.client, &self.settings, self.status.as_ref());

            tokio::pin!(shards);

            loop {
                tokio::select! {
                    // If the reshard is complete, restart the process loop.
                    shards = shards.as_mut() => {
                        self.shards = shards?;
                        break;
                    }
                    // If a task finishes and indicates that we should exit, return early.
                    Some(should_exit) = tasks.join_next() => match should_exit {
                        // If we should exit, return early.
                        Ok(Ok(true)) => return Ok(()),
                        // If the task returns an error, return it.
                        Ok(Err(error)) => return Err(error),
                        // If the task fails to join from a panic, indicate an error.
                        Err(error) if error.is_panic() => return Err(error.into()),
                        // Otherwise, just keep polling.
                        _ => continue,
                    },
                }
            }
        }
    }

    /// The task run for each spawned shard, returning whether the bot should cease execution.
    ///
    /// # Errors
    ///
    /// This function will return an error if the shard's task fails.
    pub(crate) async fn run_shard(api: Api, mut shard: Shard) -> Result<bool> {
        use twilight_gateway::StreamExt;

        let mut tasks = JoinSet::new();

        loop {
            tokio::select! {
                // If an event is given, handle it.
                event = shard.next_event(EventTypeFlags::all()) => match event {
                    // If no events are left, gracefully exit.
                    None => break,
                    // If an error occurs, log it.
                    Some(Err(error)) => warn!(async "error receiving event: {error}").await?,
                    // If an event is given, handle it.
                    Some(Ok(event)) => drop(tasks.spawn(self::event::on_event(api.clone(), event, shard.id()))),
                },
                // If a task finishes and indicates that we should exit, return early.
                Some(should_exit) = tasks.join_next() => match should_exit {
                    // If we should exit, return early.
                    Ok(Ok(true)) => return Ok(true),
                    // If the task returns an error, return it.
                    Ok(Err(error)) => return Err(error),
                    // If the task fails to join from a panic, indicate an error.
                    Err(error) if error.is_panic() => return Err(error.into()),
                    // Otherwise, just keep polling.
                    _ => {},
                },
            }
        }

        // Wait for all tasks to join.
        while tasks.join_next().await.is_some() {}

        Ok(false)
    }
}
