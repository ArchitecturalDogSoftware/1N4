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
use rand::{RngExt, rng};
use time::{Duration, OffsetDateTime};
use tracing::{debug, error, info, trace, warn};
use twilight_gateway::{Event, ShardId};
use twilight_mention::Mention;
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::{InteractionCreate, Ready};
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::InteractionResponseType;
use twilight_util::builder::message::{
    ContainerBuilder, SectionBuilder, SeparatorBuilder, TextDisplayBuilder, ThumbnailBuilder,
};

use super::api::{Api, ApiRef};
use crate::command::context::Context;
use crate::command::registry::registry;
use crate::command::resolver::{CommandOptionResolver, ModalComponentResolver, find_focused_option};
use crate::utility::category;
use crate::utility::traits::convert::{AsImage, AsLocale};
use crate::utility::traits::extension::{InteractionExt, UserExt};
use crate::utility::types::builder::ValidatedBuilder;
use crate::utility::types::custom_id::CustomId;

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
    trace!("updated event cache");

    let result: EventResult = match event {
        Event::Ready(event) => {
            debug!(version = event.version, user = %event.user.id, "received ready event");

            self::on_ready(api, event, shard_id).await
        }
        Event::InteractionCreate(event) => {
            debug!(id = %event.id, "received interaction event");

            Box::pin(self::on_interaction(api, *event)).await
        }
        Event::Resumed => {
            info!("successfully resumed");

            self::pass()
        }
        Event::GatewayHello(event) => {
            debug!(heartbeat_ms = event.heartbeat_interval, "connecting to gateway");

            self::pass()
        }
        Event::GatewayClose(reason) => {
            warn!(reason = reason.map(|frame| tracing::field::display(frame.reason)), "disconnected from gateway");

            self::pass()
        }
        Event::GatewayReconnect => {
            info!("reconnecting to gateway");

            self::pass()
        }
        event => {
            debug!(kind = ?event.kind(), "received gateway event");

            self::pass()
        }
    };

    match result {
        // Capture and log errors.
        Err(error) => {
            error!(%error, "failed to handle event");

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
#[tracing::instrument(level = "debug", name = "ready", skip_all)]
pub async fn on_ready(api: Api, event: Ready, shard_id: ShardId) -> EventResult {
    info!("connected to gateway");

    // Only shard 0 should handle command registration.
    if shard_id.number() != 0 {
        trace!("skipped ready event handler");

        return self::pass();
    }

    crate::command::registry::initialize().await?;

    if api.settings.skip_command_patch {
        info!("skipped command patching");

        return self::pass();
    }

    let client = api.client.interaction(event.application.id);
    trace!("created interaction response client");

    if let Ok(guild_id) = crate::utility::secret::development_guild_id() {
        let list = registry().await.build_and_collect::<Box<[_]>>(Some(guild_id)).await?;
        trace!(guild = %guild_id, "resolved development guild commands");

        let list = client.set_guild_commands(guild_id, &list).await?.model().await?;
        info!(commands = list.len(), "patched server commands");
    }

    if cfg!(not(debug_assertions)) {
        let list = registry().await.build_and_collect::<Box<[_]>>(None).await?;
        trace!("resolved global commands");

        let list = client.set_global_commands(&list).await?.model().await?;
        info!(commands = list.len(), "patched global commands");
    }

    self::pass()
}

/// Handles an [`InteractionCreate`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
#[tracing::instrument(level = "debug", name = "event", skip_all, fields(id = %event.id))]
pub async fn on_interaction(api: Api, event: InteractionCreate) -> EventResult {
    const TIME_WARN_THRESHOLD: Duration = Duration::seconds(1);

    info!("received interaction {}", event.display_label());

    let start_time = OffsetDateTime::now_utc();

    let result: EventResult = match event.kind {
        InteractionType::ApplicationCommand => {
            trace!("matched event type as application command");

            self::on_command(api.as_ref(), &event).await
        }
        InteractionType::MessageComponent => {
            trace!("matched event type as message component");

            self::on_component(api.as_ref(), &event).await
        }
        InteractionType::ModalSubmit => {
            trace!("matched event type as modal submission");

            self::on_modal(api.as_ref(), &event).await
        }
        InteractionType::ApplicationCommandAutocomplete => {
            trace!("matched event type as autocompletion");

            self::on_autocomplete(api.as_ref(), &event).await
        }
        kind => {
            debug!(?kind, "received unsupported event type");

            self::pass()
        }
    };

    let elapsed_time = OffsetDateTime::now_utc() - start_time;

    if elapsed_time >= TIME_WARN_THRESHOLD {
        warn!(elapsed = %elapsed_time, "interaction took longer than expected");
    } else {
        debug!(elapsed = %elapsed_time, "interaction duration is below safe threshold");
    }

    // Capture errors here to prevent duplicate logging.
    if let Err(ref error) = result {
        warn!(%error, "failed interaction {}", event.display_label());

        self::on_error(api.as_ref(), &event, error).await
    } else {
        info!("succeeded interaction {}", event.display_label());

        result
    }
}

/// Handles a command [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
#[tracing::instrument(
    level = "debug",
    name = "command",
    skip_all,
    fields(name = %match event.data {
        Some(InteractionData::ApplicationCommand(ref data)) => data.name.as_str(),
        _ => unreachable!("the given event must always be a command"),
    })
)]
pub async fn on_command(api: ApiRef<'_>, event: &Interaction) -> EventResult {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };

    let registry = registry().await;
    trace!("received access to static command registry");

    let Some(command) = registry.command(&data.name) else {
        warn!(name = %data.name, "received unrecognized command");

        bail!("missing command entry for '{}'", data.name);
    };
    trace!(name = %data.name, "resolved recognized command");

    let Some(ref callable) = command.callbacks.command else {
        bail!("missing command callback for '{}'", data.name);
    };
    trace!("resolved command callback");

    let resolver = CommandOptionResolver::new(data);

    callable.on_command(command, Context::new(api, event, data), resolver).await
}

/// Handles a component [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
#[tracing::instrument(level = "debug", name = "component", skip_all, fields(name = tracing::field::Empty))]
pub async fn on_component(api: ApiRef<'_>, event: &Interaction) -> EventResult {
    let Some(InteractionData::MessageComponent(ref data)) = event.data else {
        bail!("missing component data");
    };

    let data_id = data.custom_id.parse::<CustomId>()?;
    tracing::record_all!(tracing::Span::current(), name = %data_id.command());
    let registry = registry().await;
    trace!("received access to static command registry");

    let Some(command) = registry.command(data_id.command()) else {
        warn!(name = %data_id.command(), "received unrecognized command");

        bail!("missing command entry for '{}'", data_id.command());
    };
    trace!(name = %data_id.command(), "resolved recognized command");

    let Some(ref callable) = command.callbacks.component else {
        bail!("missing component callback for '{}'", data_id.command());
    };
    trace!("resolved component callback");

    callable.on_component(command, Context::new(api, event, data), data_id).await
}

/// Handles a modal [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
#[tracing::instrument(level = "debug", name = "modal", skip_all, fields(name = tracing::field::Empty))]
pub async fn on_modal(api: ApiRef<'_>, event: &Interaction) -> EventResult {
    let Some(InteractionData::ModalSubmit(ref data)) = event.data else {
        bail!("missing modal data");
    };

    let data_id = data.custom_id.parse::<CustomId>()?;
    tracing::record_all!(tracing::Span::current(), name = %data_id.command());
    let registry = registry().await;
    trace!("received access to static command registry");

    let Some(command) = registry.command(data_id.command()) else {
        warn!(name = %data_id.command(), "received unrecognized command");

        bail!("missing command entry for '{}'", data_id.command());
    };
    trace!(name = %data_id.command(), "resolved recognized command");

    let Some(ref callback) = command.callbacks.modal else {
        bail!("missing component callback for '{}'", data_id.command());
    };
    trace!("resolved modal callback");

    let resolver = ModalComponentResolver::new(data);

    callback.on_modal(command, Context::new(api, event, data), data_id, resolver).await
}

/// Handles an autocomplete [`Interaction`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
#[tracing::instrument(
    level = "debug",
    name = "autocomplete",
    skip_all,
    fields(name = %match event.data {
        Some(InteractionData::ApplicationCommand(ref data)) => data.name.as_str(),
        _ => unreachable!("the given event must always be a command"),
    })
)]
pub async fn on_autocomplete(api: ApiRef<'_>, event: &Interaction) -> EventResult {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };

    let registry = registry().await;
    trace!("received access to static command registry");

    let Some(command) = registry.command(&data.name) else {
        warn!(name = %data.name, "received unrecognized command");

        bail!("missing command entry for '{}'", data.name);
    };
    trace!(name = %data.name, "resolved recognized command");

    let Some(ref callback) = command.callbacks.autocomplete else {
        bail!("missing autocomplete callback for '{}'", data.name);
    };
    trace!("resolved autocomplete callback");

    let Some((name, text, kind)) = find_focused_option(&data.options) else {
        warn!("unable to resolve focused option");

        bail!("missing focused option for '{}'", data.name);
    };
    trace!(%name, ?kind, text, "resolved focused option");

    let context = Context::new(api, event, &(**data));
    let resolver = CommandOptionResolver::new(data);
    let mut choices = callback.on_autocomplete(command, context, resolver, name, text, kind).await?.to_vec();
    trace!(count = choices.len(), "determined initial choices");

    choices.dedup_by_key(|c| c.value.clone());
    choices.sort_unstable_by_key(|c| c.name.clone());
    debug!(count = choices.len(), "cleaned up choice list");

    crate::create_response!(api.client, event, struct {
        kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
        choices: choices.into_iter().take(10),
    })
    .await?;
    debug!("completed interaction");

    self::pass()
}

/// Gracefully handles an interaction error.
///
/// # Errors
///
/// This function will return an error if the logger fails to output an error log.
#[tracing::instrument(level = "debug", name = "error", skip_all)]
pub async fn on_error(api: ApiRef<'_>, event: &Interaction, error: &anyhow::Error) -> EventResult {
    debug!("received interaction error");

    if let Err(error) = self::on_error_notify_channel(api, event, error).await {
        error!(%error, "failed to output error to channel");
    }

    if let Err(error) = self::on_error_inform_user(api, event).await {
        error!(%error, "failed to inform interaction user of error");
    }

    debug!("handled error");

    self::pass()
}

/// Notifies the configured developer channel when an error occurs.
///
/// # Errors
///
/// This function will return an error if the channel could not be notified.
#[tracing::instrument(level = "trace", name = "notify_devs", skip_all)]
pub async fn on_error_notify_channel(api: ApiRef<'_>, event: &Interaction, error: &anyhow::Error) -> EventResult {
    let Ok(channel_id) = crate::utility::secret::development_channel_id() else {
        warn!("skipped channel error notification as no channel has been configured");

        return self::pass();
    };
    trace!("determined development channel identifier");

    let titles = localize!(async category::UI, "error-titles").await?.to_string();
    let titles = titles.lines().collect::<Box<[_]>>();
    let index = rng().random_range(0 .. titles.len());
    trace!("randomized container title");

    let mut container = ContainerBuilder::new().accent_color(Some(crate::utility::color::FAILURE.rgb()));

    if let Some(user) = event.author() {
        let section = SectionBuilder::new(ThumbnailBuilder::new(user.as_unfurled_media()?).try_build()?)
            .component(TextDisplayBuilder::new(format!("### {}", titles[index])).try_build()?)
            .component(TextDisplayBuilder::new(format!("`{}`", event.display_label())).try_build()?)
            .component(TextDisplayBuilder::new(format!("> {} ({})", user.display_name(), user.mention())).try_build()?)
            .try_build()?;

        container = container.component(section);
    } else {
        debug!("failed to resolve event author");

        container = container
            .component(TextDisplayBuilder::new(format!("### {}", titles[index])).try_build()?)
            .component(TextDisplayBuilder::new(format!("`{}`", event.display_label())).try_build()?);
    }
    trace!("added primary container section");

    container = container
        .component(SeparatorBuilder::new().divider(true).try_build()?)
        .component(TextDisplayBuilder::new(format!("```json\n{error}\n```")).try_build()?);
    trace!("created error display container");

    let attachment = (error.backtrace().status() == BacktraceStatus::Captured).then(|| {
        let errors = error.chain().enumerate().map(|(i, v)| format!("{} {v}", "-".repeat(i + 1))).collect::<Box<[_]>>();
        let mut lines = error.backtrace().to_string().lines().map(str::to_string).collect::<Box<[_]>>();
        debug!("captured error backtrace");

        if let Some(home_dir) = BaseDirs::new().map(|v| v.home_dir().to_path_buf()) {
            let home_dir = home_dir.to_string_lossy();

            lines.iter_mut().for_each(|v| *v = v.replace(&(*home_dir), "$HOME"));
        }
        trace!("anonymized file paths");

        let backtrace = format!("{}\n\n{}", errors.join("\n"), lines.join("\n"));

        Attachment::from_bytes("backtrace.txt".to_string(), backtrace.into_bytes(), 1)
    });

    if attachment.is_some() {
        trace!("created message attachment");
    }

    let message = api
        .client
        .create_message(channel_id)
        .flags(MessageFlags::SUPPRESS_NOTIFICATIONS | MessageFlags::IS_COMPONENTS_V2)
        .components(&[container.try_build()?.into()])
        .await?
        .model()
        .await?;
    debug!("sent initial message");

    if let Some(attachment) = attachment {
        api.client
            .create_message(channel_id)
            .flags(MessageFlags::SUPPRESS_NOTIFICATIONS)
            .reply(message.id)
            .attachments(&[attachment])
            .await?;
        debug!("replied with error backtrace");
    }

    debug!("sent developer error notification");

    self::pass()
}

/// Notifies the interaction's author when an error occurs.
///
/// # Errors
///
/// This function will return an error if the author could not be notified.
#[tracing::instrument(level = "trace", name = "notify_user", skip_all)]
pub async fn on_error_inform_user(api: ApiRef<'_>, event: &Interaction) -> EventResult {
    let Some(user) = event.author() else {
        info!("skipped user error notification as no author is present");

        return self::pass();
    };

    if matches!(event.kind, InteractionType::ApplicationCommandAutocomplete) {
        info!("skipped user error notification for autocompletion event");

        return self::pass();
    }

    let locale = match user.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    // Do our best to ensure that this is handled ephemerally.
    let _ = crate::create_response!(api.client, event, struct {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        flags: MessageFlags::EPHEMERAL,
    })
    .await;
    trace!("attempted to mark error message as ephemeral");

    let title = localize!(async(try in locale) category::UI, "error-inform-title").await?;
    let description = localize!(async(try in locale) category::UI, "error-inform-description").await?;
    let component = ContainerBuilder::new()
        .accent_color(Some(crate::utility::color::FAILURE.rgb()))
        .component(TextDisplayBuilder::new(format!("### {title}")).try_build()?)
        .component(TextDisplayBuilder::new(format!("{description}:\n`{}`", event.display_label())).try_build()?);
    trace!("created error display container");

    crate::follow_up_response!(api.client, event, struct {
        flags: MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2,
        components: &[component.try_build()?.into()],
    })
    .await?;
    debug!("sent user error notification");

    self::pass()
}
