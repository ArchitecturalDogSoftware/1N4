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

use anyhow::{anyhow, bail};
use data::{Selector, SelectorList};
use ina_localizing::localize;
use ina_storage::stored::Stored;
use tracing::{debug, trace};
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::channel::message::MessageFlags;
use twilight_model::id::Id;
use twilight_model::id::marker::RoleMarker;
use twilight_validate::component::{ACTION_ROW_COMPONENT_COUNT, COMPONENT_COUNT};

use crate::client::event::EventResult;
use crate::command::context::{Context, Visibility};
use crate::command::registry::CommandEntry;
use crate::command::resolver::CommandOptionResolver;
use crate::utility::category;
use crate::utility::traits::convert::{AsEmoji, AsLocale};
use crate::utility::types::custom_id::CustomId;

/// The command's data.
mod data;

crate::define_entry!("role", CommandType::ChatInput, struct {
    contexts: [InteractionContextType::Guild],
}, struct {
    command: on_command,
    component: on_component,
}, struct {
    create: SubCommand {
        role: Role {
            required: true,
        },
        icon: String {
            required: true,
        },
    },
    delete: SubCommand {},
    preview: SubCommand {},
    finish: SubCommand {},
});

crate::define_commands! {
    self => {
        create => on_create_command;
        delete => on_delete_command;
        preview => on_preview_command;
        finish => on_finish_command;
    }
}

crate::define_components! {
    select => on_select_component;
    remove => on_remove_component;
}

/// Executes the create command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_create_command<'ap: 'ev, 'ev>(
    _: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev CommandData>,
    resolver: CommandOptionResolver<'ev>,
) -> EventResult {
    let Some(guild_id) = context.interaction.guild_id else {
        bail!("this command must be used in a guild");
    };
    let Some(user_id) = context.interaction.author_id() else {
        bail!("this command must be used by a user");
    };
    let role_id = resolver.role_id("role")?;
    let icon = resolver.string("icon")?;
    trace!("resolved role id and icon");

    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    if icon.as_emoji().is_err() {
        debug!("an invalid icon was provided");

        let title = localize!(async(try in locale) category::UI, "role-invalid-icon").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    }
    trace!("validated role icon");

    let name = if let Some(role) = context.api.cache.role(*role_id) {
        role.name.clone()
    } else {
        let roles = context.api.client.roles(guild_id).await?.model().await?;
        let role = roles.into_iter().find_map(|r| (&r.id == role_id).then_some(r.name));

        role.ok_or_else(|| anyhow!("invalid role identifier"))?
    }
    .into_boxed_str();
    trace!("resolved role name");

    let selectors = SelectorList::async_api().read((guild_id, user_id)).await;
    let mut selectors = selectors.unwrap_or_else(|_| SelectorList::new(guild_id, user_id));
    debug!("loaded stored role selectors");

    if selectors.inner.iter().any(|s| &s.id == role_id) {
        debug!("target role is already contained within the list");

        let title = localize!(async(try in locale) category::UI, "role-selector-duplicate").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    }
    if selectors.inner.len() >= COMPONENT_COUNT * ACTION_ROW_COMPONENT_COUNT {
        debug!("role selector list is already full");

        let title = localize!(async(try in locale) category::UI, "role-selector-limit").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    }

    selectors.inner.push(Selector { id: *role_id, name, icon: icon.into() });
    debug!("created new selector");
    selectors.as_async_api().write().await?;
    debug!("wrote role selector file");

    let text = localize!(async(try in locale) category::UI, "role-selector-added").await?;

    context.success_message(text, None::<&str>).await?;
    debug!("completed interaction");

    crate::client::event::pass()
}

/// Executes the delete command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_delete_command<'ap: 'ev, 'ev>(
    entry: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev CommandData>,
    _: CommandOptionResolver<'ev>,
) -> EventResult {
    let Some(guild_id) = context.interaction.guild_id else {
        bail!("this command must be used in a guild");
    };
    let Some(user_id) = context.interaction.author_id() else {
        bail!("this command must be used by a user");
    };

    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    if !SelectorList::async_api().exists((guild_id, user_id)).await? {
        debug!("no role selectors have been stored");

        let title = localize!(async(try in locale) category::UI, "role-load-missing").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    }

    let Ok(selectors) = SelectorList::async_api().read((guild_id, user_id)).await else {
        debug!("the role selector list could not be loaded");

        let title = localize!(async(try in locale) category::UI, "role-load-failed").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    };

    let components = selectors.build(entry, component::remove::NAME, false)?;
    debug!("created message components");

    crate::follow_up_response!(context, struct {
        components: &components,
    })
    .await?;
    context.complete();
    debug!("completed interaction");

    crate::client::event::pass()
}

/// Executes the preview command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_preview_command<'ap: 'ev, 'ev>(
    entry: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev CommandData>,
    _: CommandOptionResolver<'ev>,
) -> EventResult {
    let Some(guild_id) = context.interaction.guild_id else {
        bail!("this command must be used in a guild");
    };
    let Some(user_id) = context.interaction.author_id() else {
        bail!("this command must be used by a user");
    };

    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    if !SelectorList::async_api().exists((guild_id, user_id)).await? {
        debug!("no role selectors have been stored");

        let title = localize!(async(try in locale) category::UI, "role-load-missing").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    }

    let Ok(selectors) = SelectorList::async_api().read((guild_id, user_id)).await else {
        debug!("the role selector list could not be loaded");

        let title = localize!(async(try in locale) category::UI, "role-load-failed").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    };

    let components = selectors.build(entry, component::select::NAME, true)?;
    debug!("created message components");

    context.components(components, Visibility::Ephemeral).await?;
    debug!("completed interaction");

    crate::client::event::pass()
}

/// Executes the finish command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_finish_command<'ap: 'ev, 'ev>(
    entry: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev CommandData>,
    _: CommandOptionResolver<'ev>,
) -> EventResult {
    let Some(guild_id) = context.interaction.guild_id else {
        bail!("this command must be used in a guild");
    };
    let Some(channel_id) = context.interaction.channel.as_ref().map(|c| c.id) else {
        bail!("this component must be used in a channel");
    };
    let Some(user_id) = context.interaction.author_id() else {
        bail!("this command must be used by a user");
    };

    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    if !SelectorList::async_api().exists((guild_id, user_id)).await? {
        debug!("no role selectors have been stored");

        let title = localize!(async(try in locale) category::UI, "role-load-missing").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    }

    let Ok(selectors) = SelectorList::async_api().read((guild_id, user_id)).await else {
        debug!("the role selector list could not be loaded");

        let title = localize!(async(try in locale) category::UI, "role-load-failed").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    };

    let components = selectors.build(entry, component::select::NAME, false)?;
    debug!("created message components");

    context.api.client.create_message(channel_id).flags(MessageFlags::IS_COMPONENTS_V2).components(&components).await?;
    debug!("created message");
    selectors.as_async_api().delete().await?;
    debug!("removed role selector file");

    let text = localize!(async(try in locale) category::UI, "role-finished").await?;

    context.success_message(text, None::<&str>).await?;
    debug!("completed interaction");

    crate::client::event::pass()
}

/// Executes the select component.
///
/// # Errors
///
/// This function will return an error if the component could not be executed.
async fn on_select_component<'ap: 'ev, 'ev>(
    _: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev MessageComponentInteractionData>,
    custom_id: CustomId,
) -> EventResult {
    let Some(guild_id) = context.interaction.guild_id else {
        bail!("this command must be used in a guild");
    };
    let Some(user_id) = context.interaction.author_id() else {
        bail!("this command must be used by a user");
    };
    let Some(role_id) = custom_id.get::<Id<RoleMarker>>(0).transpose()? else {
        bail!("missing role identifier data");
    };

    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let mut member = context.api.client.guild_member(guild_id, user_id).await?.model().await?;
    debug!("resolved member from user id");

    member.roles.dedup(); // Do we even need to deduplicate here?
    member.roles.sort_unstable();

    let title = if let Ok(index) = member.roles.binary_search(&role_id) {
        member.roles.remove(index);
        debug!(role = %role_id, "removed role");

        localize!(async(try in locale) category::UI, "role-removed").await?
    } else {
        member.roles.push(role_id);
        debug!(role = %role_id, "added role");

        localize!(async(try in locale) category::UI, "role-added").await?
    };

    context.api.client.update_guild_member(guild_id, user_id).roles(&member.roles).await?;
    debug!(user = %user_id, role = %role_id, "updated member roles");
    context.success_message(title, None::<&str>).await?;
    debug!("completed interaction");

    crate::client::event::pass()
}

/// Executes the remove component.
///
/// # Errors
///
/// This function will return an error if the component could not be executed.
async fn on_remove_component<'ap: 'ev, 'ev>(
    entry: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev MessageComponentInteractionData>,
    custom_id: CustomId,
) -> EventResult {
    let Some(guild_id) = context.interaction.guild_id else {
        bail!("this component must be used in a guild");
    };
    let Some(user_id) = context.interaction.author_id() else {
        bail!("this component must be used by a user");
    };
    let Some(role_id) = custom_id.get::<Id<RoleMarker>>(0).transpose()? else {
        bail!("missing role identifier data");
    };

    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    if !SelectorList::async_api().exists((guild_id, user_id)).await? {
        debug!("no role selectors have been stored");

        let title = localize!(async(try in locale) category::UI, "role-load-missing").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    }

    let Ok(mut selectors) = SelectorList::async_api().read((guild_id, user_id)).await else {
        debug!("the role selector list could not be loaded");

        let title = localize!(async(try in locale) category::UI, "role-load-failed").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    };

    if !selectors.inner.iter().any(|e| e.id == role_id) {
        debug!("the role selector no longer exists within the list");

        let title = localize!(async(try in locale) category::UI, "role-remove-missing").await?;

        context.failure_message(title, None::<&str>).await?;
        debug!("completed interaction");

        return crate::client::event::pass();
    }

    selectors.inner.retain(|e| e.id != role_id);

    if selectors.inner.is_empty() {
        selectors.as_async_api().delete().await?;
        debug!("removed role selector file");

        let title = localize!(async(try in locale) category::UI, "role-remove-emptied").await?;

        context.success_message(title, None::<&str>).await?;
    } else {
        selectors.as_async_api().write().await?;
        debug!("wrote role selector file");

        let components = selectors.build(entry, component::remove::NAME, false)?;

        context.components(components, Visibility::Ephemeral).await?;
    }
    debug!("completed interaction");

    crate::client::event::pass()
}
