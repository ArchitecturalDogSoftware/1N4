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

use anyhow::{bail, Result};
use ina_logging::{debug, info, warn};
use twilight_gateway::{Event, ShardId};
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::gateway::payload::incoming::{InteractionCreate, Ready};
use twilight_model::http::interaction::InteractionResponseType;

use super::api::Api;
use crate::command::context::Context;
use crate::command::data_id::DataId;

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
        Event::InteractionCreate(event) => self::on_interaction(api, *event, shard_id).await,
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

    crate::command::initialize().await?;

    let client = api.client.interaction(event.application.id);

    if let Ok(guild_id) = crate::utility::secret::development_guild_id() {
        let list = crate::command::registry().await.collect::<Box<[_]>>(Some(guild_id))?;
        let list = client.set_guild_commands(guild_id, &list).await?.model().await?;

        info!(async "patched {} server commands", list.len()).await?;
    }

    if cfg!(not(debug_assertions)) {
        let list = crate::command::registry().await.collect::<Box<[_]>>(None)?;
        let list = client.set_global_commands(&list).await?.model().await?;

        info!(async "patched {} global commands", list.len()).await?;
    }

    Ok(false)
}

/// Handles an [`InteractionCreate`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_interaction(api: Api, event: InteractionCreate, shard_id: ShardId) -> Result<bool> {
    info!(async "shard #{} received interaction #{}", shard_id.number(), event.id).await?;

    let result: Result<bool> = match event.kind {
        InteractionType::ApplicationCommand => self::on_command(api, &event).await,
        InteractionType::ApplicationCommandAutocomplete => self::on_autocomplete(api, &event).await,
        InteractionType::MessageComponent => self::on_component(api, &event).await,
        InteractionType::ModalSubmit => self::on_modal(api, &event).await,
        _ => Ok(false),
    };

    if let Err(ref error) = result {
        warn!(async "shard #{} failed interaction #{} - {error}", shard_id.number(), event.id).await?;
    } else {
        info!(async "shard #{} succeeded interaction #{}", shard_id.number(), event.id).await?;
    }

    result
}

/// Handles a command [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_command(api: Api, event: &Interaction) -> Result<bool> {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };
    let Some(command) = crate::command::registry().await.command(&data.name).copied() else {
        bail!("missing command entry for '{}'", data.name);
    };
    let Some(callback) = command.callbacks.command else {
        bail!("missing command callback for '{}'", data.name);
    };

    (callback)(Context::new(api.as_ref(), event, data)).await
}

/// Handles an autocomplete [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_autocomplete(api: Api, event: &Interaction) -> Result<bool> {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };
    let Some(command) = crate::command::registry().await.command(&data.name).copied() else {
        bail!("missing command entry for '{}'", data.name);
    };
    let Some(callback) = command.callbacks.autocomplete else {
        bail!("missing autocomplete callback for '{}'", data.name);
    };
    let Some((name, text, kind)) = crate::command::find_focused_option(&data.options) else {
        bail!("missing focused option for '{}'", data.name);
    };

    let choices = (callback)(Context::new(api.as_ref(), event, data), name, text, kind).await?;

    crate::create_response!(api.client, event, struct {
        kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
        choices: choices,
    })
    .await?;

    Ok(false)
}

/// Handles a component [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_component(api: Api, event: &Interaction) -> Result<bool> {
    let Some(InteractionData::MessageComponent(ref data)) = event.data else {
        bail!("missing component data");
    };
    let data_id = data.custom_id.parse::<DataId>()?;

    let Some(command) = crate::command::registry().await.command(data_id.name()).copied() else {
        bail!("missing command entry for '{}'", data_id.name());
    };
    let Some(callback) = command.callbacks.component else {
        bail!("missing component callback for '{}'", data_id.name());
    };

    (callback)(Context::new(api.as_ref(), event, data), data_id).await
}

/// Handles a modal [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_modal(api: Api, event: &Interaction) -> Result<bool> {
    let Some(InteractionData::ModalSubmit(ref data)) = event.data else {
        bail!("missing modal data");
    };
    let data_id = data.custom_id.parse::<DataId>()?;

    let Some(command) = crate::command::registry().await.command(data_id.name()).copied() else {
        bail!("missing command entry for '{}'", data_id.name());
    };
    let Some(callback) = command.callbacks.modal else {
        bail!("missing component callback for '{}'", data_id.name());
    };

    (callback)(Context::new(api.as_ref(), event, data), data_id).await
}
