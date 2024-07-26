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

use anyhow::Result;
use ina_logging::{debug, info, warn};
use twilight_gateway::{Event, ShardId};
use twilight_model::gateway::payload::incoming::Ready;

use super::api::Api;

/// Handles an event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_event(api: Api, event: Event, shard_id: ShardId) -> Result<bool> {
    api.cache.update(&event);

    let id = shard_id.number();
    let result: Result<bool> = match event {
        Event::Ready(event) => self::on_ready(api, *event, shard_id).await,
        Event::Resumed => {
            info!(async "shard #{id} successfully resumed").await?;

            Ok(false)
        }
        Event::GatewayHeartbeat(event) => {
            debug!(async "shard #{id} received heartbeat (seq. {event})").await?;

            Ok(false)
        }
        Event::GatewayHeartbeatAck => {
            debug!(async "shard #{id} received heartbeat acknowledgement").await?;

            Ok(false)
        }
        Event::GatewayHello(event) => {
            info!(async "shard #{id} connecting to gateway ({}ms)", event.heartbeat_interval).await?;

            Ok(false)
        }
        Event::GatewayClose(None) => {
            warn!(async "shard #{id} disconnected from gateway").await?;

            Ok(false)
        }
        Event::GatewayClose(Some(frame)) => {
            warn!(async "shard #{id} disconnected from gateway: {}", frame.reason).await?;

            Ok(false)
        }
        Event::GatewayReconnect => {
            info!(async "shard #{id} reconnecting to gateway").await?;

            Ok(false)
        }
        _ => Ok(false),
    };

    if let Err(error) = result {
        warn!(async "failed to handle event: {error}").await?;
    }

    Ok(false)
}

/// Handles a [`Ready`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_ready(api: Api, event: Ready, shard_id: ShardId) -> Result<bool> {
    info!(async "shard #{} connected to gateway", shard_id.number()).await?;

    let client = api.client.interaction(event.application.id);

    if let Ok(guild_id) = crate::utility::secret::development_guild_id() {
        let list = &[];
        let list = client.set_guild_commands(guild_id, list).await?.model().await?;

        info!(async "patched {} server commands", list.len()).await?;
    }

    if cfg!(not(debug_assertions)) {
        let list = &[];
        let list = client.set_global_commands(list).await?.model().await?;

        info!(async "patched {} global commands", list.len()).await?;
    }

    Ok(false)
}
