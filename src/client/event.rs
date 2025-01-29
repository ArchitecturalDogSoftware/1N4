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

use std::backtrace::BacktraceStatus;

use anyhow::bail;
use directories::BaseDirs;
use ina_localizing::localize;
use ina_logging::{debug, error, info, warn};
use rand::{Rng, rng};
use time::{Duration, OffsetDateTime};
use twilight_gateway::{Event, ShardId};
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::{InteractionCreate, Ready};
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::InteractionResponseType;
use twilight_util::builder::embed::EmbedBuilder;
use twilight_validate::embed::DESCRIPTION_LENGTH;

use super::api::{Api, ApiRef};
use crate::command::context::Context;
use crate::command::registry::registry;
use crate::command::resolver::{CommandOptionResolver, ModalFieldResolver, find_focused_option};
use crate::utility::traits::convert::{AsEmbedAuthor, AsLocale};
use crate::utility::traits::extension::InteractionExt;
use crate::utility::types::custom_id::CustomId;
use crate::utility::{category, color};

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
#[expect(clippy::missing_errors_doc, reason = "this function never returns an error")]
pub const fn pass() -> EventResult {
    Ok(EventOutput::Pass)
}

/// Returns `Ok(EventOutput::Exit)`.
#[expect(clippy::missing_errors_doc, reason = "this function never returns an error")]
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
            debug!(async "shard #{id} successfully resumed").await?;

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
            debug!(async "shard #{id} connecting to gateway ({}ms)", event.heartbeat_interval).await?;

            self::pass()
        }
        Event::GatewayClose(None) => {
            debug!(async "shard #{id} disconnected from gateway").await?;

            self::pass()
        }
        Event::GatewayClose(Some(frame)) => {
            warn!(async "shard #{id} disconnected from gateway: {}", frame.reason).await?;

            self::pass()
        }
        Event::GatewayReconnect => {
            debug!(async "shard #{id} reconnecting to gateway").await?;

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

    crate::command::registry::initialize().await?;

    if api.settings.skip_command_patch {
        info!(async "skipping command patching").await?;

        return self::pass();
    }

    let client = api.client.interaction(event.application.id);

    if let Ok(guild_id) = crate::utility::secret::development_guild_id() {
        let list = registry().await.build_and_collect::<Box<[_]>>(Some(guild_id)).await?;
        let list = client.set_guild_commands(guild_id, &list).await?.model().await?;

        info!(async "patched {} server commands", list.len()).await?;
    }

    if cfg!(not(debug_assertions)) {
        let list = registry().await.build_and_collect::<Box<[_]>>(None).await?;
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
    const TIME_WARN_THRESHOLD: Duration = Duration::seconds(1);

    info!(async "shard #{} received interaction {}", shard_id.number(), event.display_label()).await?;

    let start_time = OffsetDateTime::now_utc();

    let result: EventResult = match event.kind {
        InteractionType::ApplicationCommand => self::on_command(api.as_ref(), &event).await,
        InteractionType::MessageComponent => self::on_component(api.as_ref(), &event).await,
        InteractionType::ModalSubmit => self::on_modal(api.as_ref(), &event).await,
        InteractionType::ApplicationCommandAutocomplete => self::on_autocomplete(api.as_ref(), &event).await,
        _ => self::pass(),
    };

    let elapsed_time = OffsetDateTime::now_utc() - start_time;

    if elapsed_time >= TIME_WARN_THRESHOLD {
        warn!(async "shard #{} interaction took {elapsed_time}", shard_id.number()).await?;
    } else {
        debug!(async "shard #{} interaction took {elapsed_time}", shard_id.number()).await?;
    }

    // Capture errors here to prevent duplicate logging.
    if let Err(ref error) = result {
        warn!(async "shard #{} failed interaction {} - {error}", shard_id.number(), event.display_label()).await?;

        self::on_error(api.as_ref(), &event, error).await
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
pub async fn on_command(api: ApiRef<'_>, event: &Interaction) -> EventResult {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };

    let registry = registry().await;

    let Some(command) = registry.command(&data.name) else {
        bail!("missing command entry for '{}'", data.name);
    };
    let Some(ref callable) = command.callbacks.command else {
        bail!("missing command callback for '{}'", data.name);
    };

    let resolver = CommandOptionResolver::new(data);

    callable.on_command(command, Context::new(api, event, data), resolver).await
}

/// Handles a component [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_component(api: ApiRef<'_>, event: &Interaction) -> EventResult {
    let Some(InteractionData::MessageComponent(ref data)) = event.data else {
        bail!("missing component data");
    };

    let data_id = data.custom_id.parse::<CustomId>()?;
    let registry = registry().await;

    let Some(command) = registry.command(data_id.command()) else {
        bail!("missing command entry for '{}'", data_id.command());
    };
    let Some(ref callable) = command.callbacks.component else {
        bail!("missing component callback for '{}'", data_id.command());
    };

    callable.on_component(command, Context::new(api, event, data), data_id).await
}

/// Handles a modal [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_modal(api: ApiRef<'_>, event: &Interaction) -> EventResult {
    let Some(InteractionData::ModalSubmit(ref data)) = event.data else {
        bail!("missing modal data");
    };

    let data_id = data.custom_id.parse::<CustomId>()?;
    let registry = registry().await;

    let Some(command) = registry.command(data_id.command()) else {
        bail!("missing command entry for '{}'", data_id.command());
    };
    let Some(ref callback) = command.callbacks.modal else {
        bail!("missing component callback for '{}'", data_id.command());
    };

    let resolver = ModalFieldResolver::new(data);

    callback.on_modal(command, Context::new(api, event, data), data_id, resolver).await
}

/// Handles an autocomplete [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
pub async fn on_autocomplete(api: ApiRef<'_>, event: &Interaction) -> EventResult {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };

    let registry = registry().await;

    let Some(command) = registry.command(&data.name) else {
        bail!("missing command entry for '{}'", data.name);
    };
    let Some(ref callback) = command.callbacks.autocomplete else {
        bail!("missing autocomplete callback for '{}'", data.name);
    };
    let Some((name, text, kind)) = find_focused_option(&data.options) else {
        bail!("missing focused option for '{}'", data.name);
    };

    let context = Context::new(api, event, &(**data));
    let resolver = CommandOptionResolver::new(data);
    let mut choices = callback.on_autocomplete(command, context, resolver, name, text, kind).await?.to_vec();

    choices.dedup_by_key(|c| c.value.clone());
    choices.sort_unstable_by_key(|c| c.name.clone());

    crate::create_response!(api.client, event, struct {
        kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
        choices: choices.into_iter().take(10),
    })
    .await?;

    self::pass()
}

/// Gracefully handles an interaction error.
///
/// # Errors
///
/// This function will return an error if the logger fails to output an error log.
pub async fn on_error(api: ApiRef<'_>, event: &Interaction, error: &anyhow::Error) -> EventResult {
    if let Err(error) = self::on_error_notify_channel(api, event, error).await {
        error!(async "failed to output error to channel: {error}").await?;
    }

    if let Err(error) = self::on_error_inform_user(api, event).await {
        error!(async "failed to inform interaction user of error: {error}").await?;
    }

    self::pass()
}

/// Notifies the configured developer channel when an error occurs.
///
/// # Errors
///
/// This function will return an error if the channel could not be notified.
pub async fn on_error_notify_channel(api: ApiRef<'_>, event: &Interaction, error: &anyhow::Error) -> EventResult {
    const PREFIX: &str = "```json\n";
    const ELLIPSES: &str = "...";
    const SUFFIX: &str = "\n```";

    const MAX_DESCRIPTION_LENGTH: usize = DESCRIPTION_LENGTH - PREFIX.len() - SUFFIX.len();

    let Ok(channel_id) = crate::utility::secret::development_channel_id() else {
        warn!(async "skipping channel error notification as no channel has been configured").await?;

        return self::pass();
    };

    let titles = localize!(async category::UI, "error-titles").await?.to_string();
    let titles = titles.lines().collect::<Box<[_]>>();
    let index = rng().random_range(0 .. titles.len());

    let header = format!("`{}`\n\n", event.display_label());
    let mut description = error.to_string();

    if description.len() > MAX_DESCRIPTION_LENGTH - header.len() {
        description.truncate(MAX_DESCRIPTION_LENGTH - header.len() - ELLIPSES.len());
        description += ELLIPSES;
    }

    description = format!("{header}{PREFIX}{description}{SUFFIX}");

    let backtrace = (error.backtrace().status() == BacktraceStatus::Captured).then(|| {
        let errors = error.chain().enumerate().map(|(i, v)| format!("{} {v}", "-".repeat(i + 1))).collect::<Box<[_]>>();
        let mut lines = error.backtrace().to_string().lines().map(str::to_string).collect::<Box<[_]>>();

        if let Some(home_dir) = BaseDirs::new().map(|v| v.home_dir().to_path_buf()) {
            let home_dir = home_dir.to_string_lossy();

            lines.iter_mut().for_each(|v| *v = v.replace(&(*home_dir), "$HOME"));
        }

        format!("{}\n\n{}", errors.join("\n"), lines.join("\n"))
    });

    let mut embed = EmbedBuilder::new().color(color::FAILURE.rgb()).title(titles[index]).description(description);

    if let Some(user) = event.author() {
        embed = embed.author(user.as_embed_author()?);
    }

    let builder = api.client.create_message(channel_id).flags(MessageFlags::SUPPRESS_NOTIFICATIONS);
    let message = builder.embeds(&[embed.validate()?.build()]).await?;

    if let Some(backtrace) = backtrace {
        let attachment = Attachment::from_bytes("backtrace.txt".into(), backtrace.into_bytes(), 1);
        let message = message.model().await?;

        api.client.create_message(channel_id).reply(message.id).attachments(&[attachment]).await?;
    }

    self::pass()
}

/// Notifies the interaction's author when an error occurs.
///
/// # Errors
///
/// This function will return an error if the author could not be notified.
pub async fn on_error_inform_user(api: ApiRef<'_>, event: &Interaction) -> EventResult {
    let Some(user) = event.author() else {
        info!(async "skipping user error notification as no author is present").await?;

        return self::pass();
    };

    if matches!(event.kind, InteractionType::ApplicationCommandAutocomplete) {
        info!(async "skipping user error notification for autocompletion event").await?;

        return self::pass();
    }

    let locale = match user.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let title = localize!(async(try in locale) category::UI, "error-inform-title").await?;
    let description = localize!(async(try in locale) category::UI, "error-inform-description").await?;
    let description = format!("{description}: `{}`", event.display_label());
    let embed = EmbedBuilder::new().color(color::FAILURE.rgb()).title(title).description(description);

    // Do our best to ensure that this is handled ephemerally.
    let _ = crate::create_response!(api.client, event, struct {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        flags: MessageFlags::EPHEMERAL,
    })
    .await;

    crate::follow_up_response!(api.client, event, struct {
        embeds: &[embed.build()],
        flags: MessageFlags::EPHEMERAL,
    })
    .await?;

    self::pass()
}
