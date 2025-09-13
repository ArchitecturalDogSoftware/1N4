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
use twilight_model::channel::message::EmojiReactionType;
use twilight_model::channel::message::component::ButtonStyle;
use twilight_model::guild::{PartialMember, Permissions, Role};
use twilight_model::id::Id;
use twilight_model::id::marker::{GuildMarker, RoleMarker, UserMarker};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};
use twilight_util::permission_calculator::PermissionCalculator;

use crate::client::event::EventResult;
use crate::command::context::{Context, Visibility};
use crate::command::registry::CommandEntry;
use crate::command::resolver::CommandOptionResolver;
use crate::utility::traits::convert::{AsEmbedAuthor, AsLocale};
use crate::utility::types::builder::{ActionRowBuilder, ButtonBuilder};
use crate::utility::types::custom_id::CustomId;
use crate::utility::{category, color};

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

    let mut buffer = String::new();

    writeln!(&mut buffer, "{}", localize!((try in locale) category::UI, "help-header").await?)?;

    writeln!(&mut buffer, "### {}:\n", localize!((try in locale) category::UI, "help-global").await?)?;

    let commands = context.client().global_commands().await?.model().await?;

    self::write_command_section(&context, locale, commands, &mut buffer).await?;

    if let Some(guild_id) = context.interaction.guild_id {
        writeln!(&mut buffer, "### {}:\n", localize!((try in locale) category::UI, "help-guild").await?)?;

        let commands = context.client().guild_commands(guild_id).await?.model().await?;

        self::write_command_section(&context, locale, commands, &mut buffer).await?;
    }

    let title = localize!((try in locale) category::UI, "help-title").await?.to_string();
    let footer = localize!((try in locale) category::UI, "help-footer").await?.to_string();
    let footer = EmbedFooterBuilder::new(footer.replace("%V", env!("CARGO_PKG_VERSION"))).build();
    let color = color::BRANDING.rgb();
    let author = if let Some(user) = context.api.cache.current_user() {
        user.as_embed_author()?
    } else {
        let user = context.api.client.current_user().await?.model().await?;

        user.as_embed_author()?
    };

    let embed = EmbedBuilder::new().title(title).author(author).color(color).description(buffer).footer(footer).build();

    let command_name = command_entry.name;

    let build_information_button = ButtonBuilder::new(ButtonStyle::Secondary)
        .label(localize!((try in locale) category::UI, "help-button-build-information").await?.to_string())?
        .emoji(EmojiReactionType::Unicode { name: "ℹ️".to_string() })?
        .custom_id(CustomId::new(command_name, "build_information")?)?
        .build();
    let source_code_button = ButtonBuilder::new(ButtonStyle::Link)
        .url(env!("CARGO_PKG_REPOSITORY"))?
        .label(localize!((try in locale) category::UI, "help-button-source-code").await?.to_string())?
        .emoji(EmojiReactionType::Unicode { name: "🔗".to_string() })?
        .build();
    let licenses_button = self::attachment_button::licenses::button(locale, command_name).await?;
    let privacy_policy_button = self::attachment_button::privacy_policy::button(locale, command_name).await?;
    let security_policy_button = self::attachment_button::security_policy::button(locale, command_name).await?;

    let buttons = ActionRowBuilder::new()
        .component(build_information_button)?
        .component(source_code_button)?
        .component(licenses_button)?
        .component(privacy_policy_button)?
        .component(security_policy_button)?
        .build();

    crate::follow_up_response!(context, struct {
        components: &[buttons.into()],
        embeds: &[embed],
    })
    .await?;
    context.complete();

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

    let mut buffer = String::new();
    writeln!(buffer, "- `VERSION`: `{}`", env!("CARGO_PKG_VERSION"))?;
    writeln!(buffer, "- `FEATURES`: `{}`", info::FEATURES)?;
    writeln!(buffer, "- `COMMIT_HASH`: `{}`", info::COMMIT_HASH)?;
    writeln!(buffer, "- `TARGET_TRIPLE`: `{}`", info::TARGET_TRIPLE)?;
    writeln!(buffer, "- `PROFILE`: `{}`", info::PROFILE)?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let title = localize!((try in locale) category::UI, "help-embed-build-information-header").await?.to_string();
    let color = color::BRANDING.rgb();
    let author = if let Some(user) = context.api.cache.current_user() {
        user.as_embed_author()?
    } else {
        let user = context.api.client.current_user().await?.model().await?;

        user.as_embed_author()?
    };

    let embed = EmbedBuilder::new().title(title).author(author).color(color).description(buffer).build();
    context.embed(embed, Visibility::Ephemeral).await?;

    crate::client::event::pass()
}

/// Writes a command section into the given buffer.
///
/// # Errors
///
/// This function will return an error if a command entry could not be created.
async fn write_command_section<'ap: 'ev, 'ev, F>(
    context: &Context<'ap, 'ev, &'ev CommandData>,
    locale: Option<Locale>,
    mut commands: Vec<Command>,
    f: &mut F,
) -> Result<()>
where
    F: Write + Send,
{
    self::clean_commands(context, &mut commands);

    if commands.is_empty() {
        writeln!(f, "> *{}*", localize!((try in locale) category::UI, "help-missing").await?)?;

        return Ok(());
    }

    for command in commands {
        self::write_command(locale, command, f).await?;

        writeln!(f)?;
    }

    Ok(())
}

/// Writes a command entry into the given buffer.
///
/// # Errors
///
/// This function will return an error if the command entry could not be created.
async fn write_command<F>(locale: Option<Locale>, command: Command, f: &mut F) -> Result<()>
where
    F: Write + Send,
{
    let Command { name, kind, id, contexts, nsfw, options, .. } = command;

    let Some(id) = id else {
        return Ok(());
    };
    if kind != CommandType::ChatInput {
        return Ok(());
    }

    let localized_name_key = format!("{name}-name");
    let localized_description_key = format!("{name}-description");

    let localized_name = localize!((try in locale) category::COMMAND, localized_name_key).await?;
    let localized_description = localize!((try in locale) category::COMMAND, localized_description_key).await?;
    let has_subcommands = options.iter().any(|option| {
        //
        matches!(option.kind, CommandOptionType::SubCommand | CommandOptionType::SubCommandGroup)
    });

    if has_subcommands { write!(f, "- `/{localized_name}`") } else { write!(f, "- </{name}:{id}>") }?;

    let mut flags = Vec::with_capacity(3);

    if has_subcommands {
        flags.push(localize!((try in locale) category::UI, "help-tag-subcommands").await?);
    }
    if contexts.is_some_and(|v| v.contains(&InteractionContextType::BotDm)) {
        flags.push(localize!((try in locale) category::UI, "help-tag-dms").await?);
    }
    if nsfw.unwrap_or(false) {
        flags.push(localize!((try in locale) category::UI, "help-tag-nsfw").await?);
    }

    if !flags.is_empty() {
        let flags = flags.into_iter().map(|t| t.to_string()).collect::<Box<[_]>>();

        write!(f, " - *{}*", flags.join(", "))?;
    }

    write!(f, "\n> {localized_description}").map_err(Into::into)
}

/// Cleans up a list of commands.
fn clean_commands<'ap: 'ev, 'ev>(_: &Context<'ap, 'ev, &'ev CommandData>, commands: &mut [Command]) {
    // TODO: See if there's a way to improve this; currently it hides *too many* commands.
    //
    // if let Some((guild_id, user_id)) = context.interaction.guild_id.zip(context.interaction.author_id()) {
    // let member = context.interaction.member.as_ref();
    // let permissions = self::get_member_permissions(context, guild_id, user_id, member).await?;
    //
    // commands.retain(|c| c.default_member_permissions.is_none_or(|p| p.contains(permissions)));
    // };

    commands.sort_unstable_by_key(|c| c.name.clone());
}

/// Returns a given member's permissions.
///
/// # Errors
///
/// This function will return an error if the member's permissions could not be resolved.
#[expect(dead_code, reason = "this is not currently used")]
async fn get_member_permissions<'ap: 'ev, 'ev>(
    context: &Context<'ap, 'ev, &'ev CommandData>,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    member: Option<&PartialMember>,
) -> Result<Permissions> {
    #[inline]
    fn get_role_permissions(roles: &[Role], role_id: Id<RoleMarker>) -> Permissions {
        roles.iter().find_map(|r| (r.id == role_id).then_some(r.permissions)).unwrap_or(Permissions::empty())
    }

    if let Some(permissions) = member.and_then(|m| m.permissions) {
        return Ok(permissions);
    }

    let owner_id = if let Some(guild) = context.api.cache.guild(guild_id) {
        guild.owner_id()
    } else {
        context.api.client.guild(guild_id).await?.model().await?.owner_id
    };

    let guild_roles = context.api.client.roles(guild_id).await?.model().await?;
    let everyone_role = get_role_permissions(&guild_roles, guild_id.cast());
    let member_roles: Box<[_]> = if let Some(member) = member {
        member.roles.iter().map(|&r| (r, get_role_permissions(&guild_roles, r))).collect()
    } else if let Some(member) = context.api.cache.member(guild_id, user_id) {
        member.roles().iter().map(|&r| (r, get_role_permissions(&guild_roles, r))).collect()
    } else {
        let member = context.api.client.guild_member(guild_id, user_id).await?.model().await?;

        member.roles.into_iter().map(|r| (r, get_role_permissions(&guild_roles, r))).collect()
    };

    let calculator = PermissionCalculator::new(guild_id, user_id, everyone_role, &member_roles).owner_id(owner_id);

    Ok(if let Some(ref channel) = context.interaction.channel {
        // TODO: See if there's a way to check overridden permissions instead.
        calculator.in_channel(channel.kind, channel.permission_overwrites.as_deref().unwrap_or(&[]))
    } else {
        calculator.root()
    })
}
