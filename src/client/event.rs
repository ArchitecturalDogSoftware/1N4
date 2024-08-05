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

use anyhow::bail;
use ina_logging::{debug, info, warn};
use twilight_gateway::{Event, ShardId};
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::gateway::payload::incoming::{InteractionCreate, Ready};
use twilight_model::http::interaction::InteractionResponseType;

use super::api::Api;
use crate::command::context::Context;
use crate::utility::traits::extension::InteractionExt;
use crate::utility::types::id::CustomId;

/// A result returned by an event handler.
pub type EventResult = std::result::Result<EventOutput, anyhow::Error>;

/// An output returned by an event handler.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum EventOutput {
    /// Continue running.
    Pass,
    /// Exit the application.
    Exit,
}

/// Returns `Ok(EventOutput::Pass)`.
#[allow(clippy::missing_errors_doc)]
pub const fn pass() -> EventResult {
    Ok(EventOutput::Pass)
}

/// Returns `Ok(EventOutput::Exit)`.
#[allow(clippy::missing_errors_doc)]
pub const fn exit() -> EventResult {
    Ok(EventOutput::Exit)
}

/// Handles an event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_event(api: Api, event: Event, shard_id: ShardId) -> EventResult {
    api.cache.update(&event);

    let id = shard_id.number();
    let result: EventResult = match event {
        Event::Ready(event) => self::on_ready(api, *event, shard_id).await,
        Event::InteractionCreate(event) => self::on_interaction(api, *event, shard_id).await,
        Event::Resumed => {
            info!(async "shard #{id} successfully resumed").await?;

            self::pass()
        }
        Event::GatewayHeartbeat(event) => {
            debug!(async "shard #{id} received heartbeat (seq. {event})").await?;

            self::pass()
        }
        Event::GatewayHeartbeatAck => {
            debug!(async "shard #{id} received heartbeat acknowledgement").await?;

            self::pass()
        }
        Event::GatewayHello(event) => {
            info!(async "shard #{id} connecting to gateway ({}ms)", event.heartbeat_interval).await?;

            self::pass()
        }
        Event::GatewayClose(None) => {
            warn!(async "shard #{id} disconnected from gateway").await?;

            self::pass()
        }
        Event::GatewayClose(Some(frame)) => {
            warn!(async "shard #{id} disconnected from gateway: {}", frame.reason).await?;

            self::pass()
        }
        Event::GatewayReconnect => {
            info!(async "shard #{id} reconnecting to gateway").await?;

            self::pass()
        }
        _ => self::pass(),
    };

    match result {
        // Capture and log errors.
        Err(error) => {
            warn!(async "failed to handle event: {error}").await?;

            self::pass()
        }
        result => result,
    }
}

/// Handles a [`Ready`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_ready(api: Api, event: Ready, shard_id: ShardId) -> EventResult {
    info!(async "shard #{} connected to gateway", shard_id.number()).await?;

    // Only shard 0 should handle command registration.
    if shard_id.number() != 0 {
        return self::pass();
    }

    crate::command::initialize().await?;

    let client = api.client.interaction(event.application.id);

    if let Ok(guild_id) = crate::utility::secret::development_guild_id() {
        let list = crate::command::registry().await.collect::<Box<[_]>>(Some(guild_id)).await?;
        let list = client.set_guild_commands(guild_id, &list).await?.model().await?;

        info!(async "patched {} server commands", list.len()).await?;
    }

    if cfg!(not(debug_assertions)) {
        let list = crate::command::registry().await.collect::<Box<[_]>>(None).await?;
        let list = client.set_global_commands(&list).await?.model().await?;

        info!(async "patched {} global commands", list.len()).await?;
    }

    self::pass()
}

/// Handles an [`InteractionCreate`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_interaction(api: Api, event: InteractionCreate, shard_id: ShardId) -> EventResult {
    info!(async "shard #{} received interaction {}", shard_id.number(), event.display_label()).await?;

    let result: EventResult = match event.kind {
        InteractionType::ApplicationCommand => self::on_command(api, &event).await,
        InteractionType::ApplicationCommandAutocomplete => self::on_autocomplete(api, &event).await,
        InteractionType::MessageComponent => self::on_component(api, &event).await,
        InteractionType::ModalSubmit => self::on_modal(api, &event).await,
        _ => self::pass(),
    };

    // Capture errors here to prevent duplicate logging.
    if let Err(ref error) = result {
        warn!(async "shard #{} failed interaction {} - {error}", shard_id.number(), event.display_label()).await?;

        self::pass()
    } else {
        info!(async "shard #{} succeeded interaction {}", shard_id.number(), event.display_label()).await?;

        result
    }
}

/// Handles a command [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_command(api: Api, event: &Interaction) -> EventResult {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };

    let registry = &crate::command::registry().await;

    let Some(command) = registry.command(&data.name) else {
        bail!("missing command entry for '{}'", data.name);
    };
    let Some(ref callable) = command.callbacks.command else {
        bail!("missing command callback for '{}'", data.name);
    };

    callable.on_command(Context::new(api.as_ref(), event, data)).await
}

/// Handles a component [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_component(api: Api, event: &Interaction) -> EventResult {
    let Some(InteractionData::MessageComponent(ref data)) = event.data else {
        bail!("missing component data");
    };

    let data_id = data.custom_id.parse::<CustomId>()?;
    let registry = &crate::command::registry().await;

    let Some(command) = registry.command(data_id.name()) else {
        bail!("missing command entry for '{}'", data_id.name());
    };
    let Some(ref callable) = command.callbacks.component else {
        bail!("missing component callback for '{}'", data_id.name());
    };

    callable.on_component(Context::new(api.as_ref(), event, data), data_id).await
}

/// Handles a modal [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_modal(api: Api, event: &Interaction) -> EventResult {
    let Some(InteractionData::ModalSubmit(ref data)) = event.data else {
        bail!("missing modal data");
    };

    let data_id = data.custom_id.parse::<CustomId>()?;
    let registry = &crate::command::registry().await;

    let Some(command) = registry.command(data_id.name()) else {
        bail!("missing command entry for '{}'", data_id.name());
    };
    let Some(ref callback) = command.callbacks.modal else {
        bail!("missing component callback for '{}'", data_id.name());
    };

    callback.on_modal(Context::new(api.as_ref(), event, data), data_id).await
}

/// Handles an autocomplete [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_autocomplete(api: Api, event: &Interaction) -> EventResult {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };

    let registry = &crate::command::registry().await;

    let Some(command) = registry.command(&data.name) else {
        bail!("missing command entry for '{}'", data.name);
    };
    let Some(ref callback) = command.callbacks.autocomplete else {
        bail!("missing autocomplete callback for '{}'", data.name);
    };
    let Some((name, text, kind)) = crate::command::find_focused_option(&data.options) else {
        bail!("missing focused option for '{}'", data.name);
    };

    let context = Context::new(api.as_ref(), event, &(**data));
    let mut choices = callback.on_autocomplete(context, name, text, kind).await?.to_vec();

    choices.dedup_by_key(|c| c.value.clone());
    choices.sort_unstable_by_key(|c| c.name.clone());

    crate::create_response!(api.client, event, struct {
        kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
        choices: choices.into_iter().take(10),
    })
    .await?;

    self::pass()
}
