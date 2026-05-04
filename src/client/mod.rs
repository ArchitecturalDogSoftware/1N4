// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024-2026 Jaxydog
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

use std::num::NonZero;
use std::time::{Duration, Instant};

use anyhow::Result;
use rand::{RngExt, rng};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;
use tokio_stream::{StreamExt, StreamMap};
use tracing::{Instrument, debug, error, info, trace, trace_span, warn};
use twilight_gateway::{Config, ConfigBuilder, EventTypeFlags, Intents, Shard};
use twilight_http::Client;
use twilight_model::gateway::OpCode;
use twilight_model::gateway::connection_info::BotConnectionInfo;
use twilight_model::gateway::payload::outgoing::UpdatePresence;
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::presence::{ActivityType, MinimalActivity, Status};

use self::api::Api;
use self::event::{EventOutput, EventResult};
use self::settings::Settings;

/// Provides an API structure to be passed between functions.
pub mod api;
/// Provides an API for handling events.
pub mod event;
/// Defines the client's settings.
pub mod settings;

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

/// The bot's status definition schema.
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusList {
    /// The bot's testing status definitions.
    pub testing: Box<[StatusDefinition]>,
    /// The bot's release status definitions.
    pub release: Box<[StatusDefinition]>,
}

impl StatusList {
    /// Returns a reference to a random status from this [`StatusList`].
    #[must_use]
    pub fn random(&self) -> &StatusDefinition {
        let list = if cfg!(debug_assertions) { &self.testing } else { &self.release };

        &list[rng().random_range(0 .. list.len())]
    }
}

/// A status definition.
#[non_exhaustive]
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

impl Default for StatusDefinition {
    fn default() -> Self {
        const STATUS: Status = if cfg!(debug_assertions) { Status::Idle } else { Status::Online };

        Self { status: STATUS, activity: None, content: None, link: None }
    }
}

/// Handles a task's join result.
///
/// # Examples
///
/// ```
/// match_join_result!(join_result, "event", EventOutput::Exit);
/// ```
macro_rules! match_join_result {
    ($result:expr, $type:literal, $exit:expr) => {
        match $result {
            // Just keep polling if instructed to pass.
            Ok(Ok(EventOutput::Pass)) => trace!("received pass signal from event"),
            // If we should exit, return early.
            Ok(Ok(EventOutput::Exit)) => {
                trace!("received exit signal from event");
                return Ok($exit)
            }
            // If the task returns an error, return it.
            Ok(Err(error)) => {
                warn!("received error from event");
                return Err(error)
            }
            // If the task fails to join from a panic, indicate an error.
            Err(error) if error.is_panic() => {
                error!("received panic from event");
                return Err(error.into())
            }
            // If the task fails to join from a panic, indicate an error.
            Err(error) => error!(kind = %$type, %error, "task failed to join"),
        }
    };
}

/// The bot's instance.
#[non_exhaustive]
#[derive(Debug)]
pub struct Instance {
    /// The canonical API instance.
    api: Api,
    /// The bot instance's created shards.
    shards: Box<[Shard]>,
    /// The bot's configured status list.
    status: Option<StatusList>,
}

impl Instance {
    /// Creates a new [`Instance`].
    ///
    /// # Errors
    ///
    /// This function will return an error if an [`Instance`] cannot be created.
    #[tracing::instrument(level = "debug", name = "new_client", skip_all)]
    pub async fn new(settings: Settings) -> Result<Self> {
        // If this fails, it means that the provider was already set, meaning that we can safely ignore it.
        // Just in case this *does* cause an issue one day, we output a warning log.
        if rustls::crypto::ring::default_provider().install_default().is_err() {
            warn!("cryptographic provider has already been set");
        }
        debug!("installed default cryptographic provider");

        let discord_token = crate::utility::secret::discord_token()?;
        let client = Client::new(discord_token.to_string());
        trace!("created twilight http client");
        let status = Self::new_status(&settings).await?;
        trace!(count = status.as_ref().map_or(0, |l| l.testing.len() + l.release.len()), "created status list");
        let config = Self::new_config(discord_token.to_string(), status.as_ref())?;
        trace!("created discord shard configuration");
        let shards = Self::new_shards(&client, config, &settings).await?;
        trace!(count = shards.len(), "created discord shard(s)");

        debug!(shards = shards.len(), "created new client instance");

        Ok(Self { api: Api::new(settings, client), shards, status })
    }

    /// Creates a new [`StatusList`], returning [`None`] if a file could not be found.
    ///
    /// # Errors
    ///
    /// This function will return an error if the [`StatusList`] could not be deserialized.
    pub async fn new_status(settings: &Settings) -> Result<Option<StatusList>> {
        let path = &(*settings.status_file);

        if !tokio::fs::try_exists(path).await? {
            error!(?path, "unable to locate status file");

            return Ok(None);
        }

        let data = tokio::fs::read_to_string(path).await?;
        trace!(?path, "read status file");

        let data = toml::from_str::<StatusList>(&data)?;
        trace!("parsed status file");

        debug!(count = data.testing.len() + data.release.len(), "loaded status entries");

        Ok(Some(data))
    }

    /// Creates a new [`Config`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the [`Config`] could not be created.
    pub fn new_config(token: String, status: Option<&StatusList>) -> Result<Config> {
        let payload = if let Some(status) = status {
            Self::get_status(status.random())?
        } else {
            warn!("status file failed to load, using default status");

            Self::get_status(&StatusDefinition::default())?
        };

        debug!(intents = self::INTENTS.bits(), "created new discord api configuration");

        Ok(ConfigBuilder::new(token, self::INTENTS).presence(payload).build())
    }

    /// Creates a new list of shards.
    ///
    /// # Errors
    ///
    /// This function will return an error if the shards could not be created.
    pub async fn new_shards(client: &Client, config: Config, settings: &Settings) -> Result<Box<[Shard]>> {
        let shards: Box<[_]> = if let Some(count) = settings.shards.map(NonZero::get) {
            twilight_gateway::create_iterator(0 .. count, count, config, |_, b| b.build()).collect()
        } else {
            debug!("shard count was not specified at the command-line, using discord recommended count");
            twilight_gateway::create_recommended(client, config, |_, b| b.build()).await?.collect()
        };

        if settings.shards.is_some() {
            debug!(count = shards.len(), "created client shards (user-specified)");
        } else {
            debug!(count = shards.len(), "created client shards (recommended)");
        }

        Ok(shards)
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
            _ => {
                warn!(?definition, "an invalid status definition was provided");

                MinimalActivity { kind: ActivityType::Custom, name: String::new(), url: None }
            }
        };

        UpdatePresencePayload::new(vec![activity.into()], false, None, definition.status).map_err(Into::into)
    }

    /// Returns a [`Duration`] representing the time delay between re-sharding the bot process.
    #[expect(clippy::cast_possible_truncation, reason = "this is fine as we round before casting")]
    #[expect(clippy::cast_sign_loss, reason = "buckets cannot be negative")]
    #[expect(clippy::cast_precision_loss, reason = "there will never be enough shards for this to matter")]
    pub(crate) fn get_shard_timeout(connection: &BotConnectionInfo) -> Duration {
        const DAY: Duration = Duration::from_hours(24);

        let timeout = Duration::from_millis(connection.session_start_limit.reset_after);
        let refills = connection.shards / connection.session_start_limit.remaining;
        let sessions = u64::from(connection.session_start_limit.total);
        let buckets = (connection.shards as f32) / f32::from(connection.session_start_limit.max_concurrency);
        let buckets = buckets.round() as u64;

        timeout * u32::from(refills > 0)
            + (refills - 1) * DAY // _
            + Duration::from_secs(5 * buckets % sessions)
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
        tokio::time::sleep(Duration::from_hours(settings.reshard_interval.get())).await;
        debug!("started reshard process");

        let connection = client.gateway().authed().await?.model().await?;
        trace!("retrieved client gateway connection information");

        let discord_token = crate::utility::secret::discord_token()?.to_string();
        let config = Self::new_config(discord_token, status)?;
        let mut shards = Self::new_shards(client, config, settings).await?;
        trace!("created new shards");

        let reshard_timeout = tokio::time::sleep(Self::get_shard_timeout(&connection));

        tokio::pin!(reshard_timeout);

        std::future::poll_fn(|cx| {
            _ = reshard_timeout.as_mut().poll(cx);

            std::task::Poll::Ready(())
        })
        .await;
        trace!("started identification timeout");

        // Attempt to identify early to make the swap cleaner.
        let mut identified = vec![false; shards.len()].into_boxed_slice();
        let mut shard_stream = shards.iter_mut().map(|s| (s.id(), s)).collect::<StreamMap<_, _>>();

        loop {
            let identified_count = identified.iter().filter(|b| **b).count();

            tokio::select! {
                // Exit early if we time out and at least 75% of the shards are identified.
                () = &mut reshard_timeout, if identified_count >= (identified.len() * 3) / 4 => {
                    warn!("not all shards were identified before the computed timeout");

                    break
                },
                Some((shard_id, result)) = shard_stream.next() => {
                    if let Err(error) = result {
                        error!(%error, "failed to identify shard");

                        continue;
                    }

                    let Some(shard) = shard_stream.values().find(|s| s.id() == shard_id) else {
                        unreachable!("the value is guaranteed to be present within this stream");
                    };

                    identified[shard_id.number() as usize] = shard.state().is_identified();
                    trace!(id = shard_id.number(), "identified shard");
                }
            }
        }

        info!("resharded client connection");

        Ok(shards)
    }

    /// Runs the bot application.
    ///
    /// # Errors
    ///
    /// This function will return an error if the instance encounters an unhandled exception.
    #[tracing::instrument(level = "trace", name = "client", skip_all)]
    pub async fn run(mut self) -> Result<()> {
        info!("started client process");

        loop {
            let mut senders = Vec::with_capacity(self.shards.len());
            let mut tasks = JoinSet::new();
            debug!("started primary client loop");

            for shard in self.shards {
                senders.push(shard.sender());

                tasks.spawn(Self::run_shard(self.api.clone(), shard));
            }
            trace!("spawned shard processes");

            let shards = Self::try_reshard(&self.api.client, &self.api.settings, self.status.as_ref());

            tokio::pin!(shards);

            let duration = Duration::from_mins(self.api.settings.status_interval.get());
            let mut status_interval = tokio::time::interval_at((Instant::now() + duration).into(), duration);

            loop {
                tokio::select! {
                    // If the reshard is complete, restart the process loop.
                    shards = shards.as_mut() => {
                        self.shards = shards?;
                        debug!("finished reshard, restarting primary client loop");

                        break;
                    }
                    // Update the bot's status if the interval has elapsed.
                    _ = status_interval.tick() => {
                        let payload = if let Some(ref status) = self.status {
                            Self::get_status(status.random())?
                        } else {
                            warn!("status file failed to load, using default status");

                            Self::get_status(&StatusDefinition::default())?
                        };
                        trace!("determined new client presence");

                        let presence = UpdatePresence {
                            op: OpCode::PresenceUpdate,
                            d: payload,
                        };

                        for sender in senders.iter().filter(|c| !c.is_closed()) {
                            sender.command(&presence)?;
                        }
                        debug!("updated client presence");
                    }
                    // If a task finishes and indicates that we should exit, return early.
                    Some(result) = tasks.join_next() => match_join_result!(result, "shard", ()),
                }
            }
        }
    }

    /// The task run for each spawned shard, returning whether the bot should cease execution.
    ///
    /// # Errors
    ///
    /// This function will return an error if the shard's task fails.
    #[tracing::instrument(level = "debug", name = "shard", skip_all, fields(id = shard.id().number()))]
    pub(crate) async fn run_shard(api: Api, mut shard: Shard) -> EventResult {
        use twilight_gateway::StreamExt;

        trace!("spawned shard process");

        let mut tasks = JoinSet::new();

        loop {
            tokio::select! {
                // If an event is given, handle it.
                event = shard.next_event(EventTypeFlags::all()) => match event {
                    // If an event is given, handle it.
                    Some(Ok(event)) => {
                        let span = trace_span!("task");
                        tasks.spawn(self::event::on_event(api.clone(), event, shard.id()).instrument(span));
                        trace!("spawned child task to handle incoming event");
                    },
                    // If an error occurs, log it.
                    Some(Err(error)) => error!(%error, "error receiving event"),
                    // If no events are left, gracefully exit.
                    None => {
                        trace!("no events remaining, exiting shard loop");

                        break;
                    },
                },
                // If a task finishes and indicates that we should exit, return early.
                Some(result) = tasks.join_next() => match_join_result!(result, "event", EventOutput::Exit),
            }
        }
        debug!("exited shard loop");

        // Wait for all tasks to join naturally.
        while let Some(result) = tasks.join_next().await {
            match_join_result!(result, "event", EventOutput::Exit);
            trace!("joined child task");
        }
        debug!("stopped shard process");

        self::event::pass()
    }
}
