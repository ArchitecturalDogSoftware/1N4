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

use std::fmt::Write;

use anyhow::Result;
use ina_localizing::locale::Locale;
use ina_localizing::localize;
use twilight_model::application::command::{Command, CommandOptionType, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::channel::message::Component;
use twilight_model::channel::message::component::{Button, ButtonStyle, UnfurledMediaItem};
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;
use twilight_util::builder::message::{
    ButtonBuilder, ContainerBuilder, SectionBuilder, SeparatorBuilder, TextDisplayBuilder, ThumbnailBuilder,
};

use crate::client::event::EventResult;
use crate::command::context::{Context, Visibility};
use crate::command::registry::CommandEntry;
use crate::command::resolver::CommandOptionResolver;
use crate::utility::category;
use crate::utility::traits::convert::{AsImage, AsLocale};
use crate::utility::traits::extension::UnfurledMediaItemExt;
use crate::utility::types::builder::ValidatedBuilder;
use crate::utility::types::custom_id::CustomId;

mod attachment_button;

// [`crate::define_components`] expects identifiers, not paths, so we need to have these directly
// in scope.
use self::attachment_button::licenses::on_component as on_licenses_component;
use self::attachment_button::privacy_policy::on_component as on_privacy_policy_component;
use self::attachment_button::security_policy::on_component as on_security_policy_component;

crate::define_entry!("help", CommandType::ChatInput, struct {
    contexts: [InteractionContextType::Guild, InteractionContextType::BotDm],
}, struct {
    command: on_command,
    component: on_component,
}, struct {});

crate::define_components! {
    build_information => on_build_information_component;
    licenses => on_licenses_component;
    privacy_policy => on_privacy_policy_component;
    security_policy => on_security_policy_component;
}

/// Executes the command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_command<'ap: 'ev, 'ev>(
    command_entry: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev CommandData>,
    _: CommandOptionResolver<'ev>,
) -> EventResult {
    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let avatar_url = if let Some(user) = context.api.cache.current_user() {
        user.as_image_url()?
    } else {
        let user = context.api.client.current_user().await?.model().await?;

        user.as_image_url()?
    };

    let title = localize!(async(try in locale) category::UI, "help-title").await?.to_string();
    let header = localize!(async(try in locale) category::UI, "help-header").await?;

    let section = SectionBuilder::new(ThumbnailBuilder::new(UnfurledMediaItem::url(avatar_url)).try_build()?)
        .component(TextDisplayBuilder::new(format!("### {title}")).try_build()?)
        .component(TextDisplayBuilder::new(header).try_build()?);

    let mut container = ContainerBuilder::new()
        .accent_color(Some(crate::utility::color::BRANDING.rgb()))
        .component(section.try_build()?)
        .component(SeparatorBuilder::new().try_build()?)
        .component(self::create_command_section(context, locale, None).await?);

    if let Some(guild_id) = context.interaction.guild_id {
        container = container.component(self::create_command_section(context, locale, Some(guild_id)).await?);
    }

    let command_name = command_entry.name;

    let build_information_button = ButtonBuilder::new(ButtonStyle::Secondary)
        .label(localize!(async(try in locale) category::UI_BUTTON, "help-view").await?.to_string())
        .custom_id(CustomId::new(command_name, "build_information")?)
        .try_build()?;
    let source_code_button = ButtonBuilder::new(ButtonStyle::Link)
        .url(env!("CARGO_PKG_REPOSITORY"))
        .label(localize!(async(try in locale) category::UI_BUTTON, "help-open").await?.to_string())
        .try_build()?;
    let licenses_button = self::attachment_button::licenses::button(locale, command_name).await?;
    let privacy_policy_button = self::attachment_button::privacy_policy::button(locale, command_name).await?;
    let security_policy_button = self::attachment_button::security_policy::button(locale, command_name).await?;

    let footer = localize!(async(try in locale) category::UI, "help-footer").await?.to_string();
    let footer = footer.split('\n').map(|s| format!("-# {s}")).collect::<Vec<_>>().join("\n");

    container = container
        .component(SeparatorBuilder::new().try_build()?)
        .component(self::create_button_section(locale, "source-code", "url", source_code_button).await?)
        .component(self::create_button_section(locale, "build-information", "reply", build_information_button).await?)
        .component(self::create_button_section(locale, "licenses", "reply", licenses_button).await?)
        .component(self::create_button_section(locale, "privacy-policy", "reply", privacy_policy_button).await?)
        .component(self::create_button_section(locale, "security-policy", "reply", security_policy_button).await?)
        .component(SeparatorBuilder::new().try_build()?)
        .component(TextDisplayBuilder::new(footer.replace("%V", env!("CARGO_PKG_VERSION"))).try_build()?);

    context.components([container.try_build()?], Visibility::Ephemeral).await?;

    crate::client::event::pass()
}

/// Executes the build information component, sending an embed listing properties of this build of
/// 1N4, such as the version and enabled features.
///
/// # Errors
///
/// This function will return an error if the component could not be executed.
async fn on_build_information_component<'ap: 'ev, 'ev>(
    _: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev MessageComponentInteractionData>,
    _: CustomId,
) -> EventResult {
    mod info {
        include!(concat!(env!("OUT_DIR"), "/build_info.rs"));
    }

    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let mut buffer = String::new();

    writeln!(&mut buffer, "- `VERSION`: `{}`", env!("CARGO_PKG_VERSION"))?;
    writeln!(&mut buffer, "- `FEATURES`: `{}`", info::FEATURES)?;
    writeln!(&mut buffer, "- `COMMIT_HASH`: `{}`", info::COMMIT_HASH)?;
    writeln!(&mut buffer, "- `TARGET_TRIPLE`: `{}`", info::TARGET_TRIPLE)?;
    writeln!(&mut buffer, "- `PROFILE`: `{}`", info::PROFILE)?;

    let title = localize!(async(try in locale) category::UI, "help-build-information-header").await?;
    let container = ContainerBuilder::new()
        .accent_color(Some(crate::utility::color::BRANDING.rgb()))
        .component(TextDisplayBuilder::new(format!("### {title}")).try_build()?)
        .component(SeparatorBuilder::new().try_build()?)
        .component(TextDisplayBuilder::new(buffer).try_build()?);

    context.components([container.try_build()?], Visibility::Ephemeral).await?;

    crate::client::event::pass()
}

/// Creates a component that displays all available command entries.
///
/// # Errors
///
/// This function will return an error if a command entry could not be created.
async fn create_command_section<'ap: 'ev, 'ev>(
    context: Context<'ap, 'ev, &'ev CommandData>,
    locale: Option<Locale>,
    guild_id: Option<Id<GuildMarker>>,
) -> Result<Component> {
    let mut section_content = String::new();

    let (title, mut commands) = if let Some(guild_id) = guild_id {
        (
            localize!(async(try in locale) category::UI, "help-global").await?,
            context.client().guild_commands(guild_id).await?.model().await?,
        )
    } else {
        (
            localize!(async(try in locale) category::UI, "help-guild").await?,
            context.client().global_commands().await?.model().await?,
        )
    };

    writeln!(&mut section_content, "**{title}:**")?;

    // TODO: See if there's any way to reliably trim commands that the calling user doesn't have access to.

    if commands.is_empty() {
        let missing_text = localize!(async(try in locale) category::UI, "help-missing").await?;

        write!(&mut section_content, "> *{missing_text}*")?;
    } else {
        commands.sort_unstable_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

        for command in commands {
            let Some(command_content) = self::create_command_entry(locale, command).await? else { continue };

            writeln!(&mut section_content, "{command_content}")?;
        }
    }

    Ok(TextDisplayBuilder::new(section_content).try_build()?.into())
}

/// Creates a string that displays a command entry.
///
/// # Errors
///
/// This function will return an error if the command entry could not be created.
async fn create_command_entry(locale: Option<Locale>, command: Command) -> Result<Option<String>> {
    // If this is none, it means that the command has not been registered and we should skip it.
    let Some(command_id) = command.id else { return Ok(None) };

    if command.kind != CommandType::ChatInput {
        return Ok(None);
    }

    let mut content = String::new();
    let mut command_flags = Vec::<String>::with_capacity(3);

    if command.options.iter().any(|option| {
        //
        matches!(option.kind, CommandOptionType::SubCommand | CommandOptionType::SubCommandGroup)
    }) {
        command_flags.push(localize!(async(try in locale) category::UI, "help-tag-subcommands").await?.into());

        let localized_name_key = format!("{}-name", command.name);
        let localized_name = localize!(async(try in locale) category::COMMAND, localized_name_key).await?;

        write!(&mut content, "- `/{localized_name}`")?;
    } else {
        write!(&mut content, "- </{}:{command_id}>", command.name)?;
    }

    if command.contexts.is_some_and(|context| context.contains(&InteractionContextType::BotDm)) {
        command_flags.push(localize!(async(try in locale) category::UI, "help-tag-dms").await?.into());
    }
    if command.nsfw.unwrap_or(false) {
        command_flags.push(localize!(async(try in locale) category::UI, "help-tag-nsfw").await?.into());
    }

    if !command_flags.is_empty() {
        write!(&mut content, " - *{}*", command_flags.join(", "))?;
    }

    let localized_description_key = format!("{}-description", command.name);
    let localized_description = localize!(async(try in locale) category::COMMAND, localized_description_key).await?;

    write!(&mut content, "\n> {localized_description}")?;

    Ok(Some(content))
}

/// Creates a new section for the given button.
///
/// # Errors
///
/// This function will return an error if the section could not be created.
async fn create_button_section(
    locale: Option<Locale>,
    text_key: &str,
    action_key: &str,
    button: Button,
) -> Result<Component> {
    let text = localize!(async(try in locale) category::UI_BUTTON, format!("help-label-{text_key}")).await?;
    let action = localize!(async(try in locale) category::UI_BUTTON, format!("help-action-{action_key}")).await?;

    let text_display = TextDisplayBuilder::new(format!("{text}\n-# {action}")).try_build()?;

    Ok(SectionBuilder::new(button).component(text_display).try_build()?.into())
}
