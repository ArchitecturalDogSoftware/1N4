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

use std::fmt::Write;

use anyhow::Result;
use ina_localizing::locale::Locale;
use ina_localizing::localize;
use rand::{Rng, thread_rng};
use twilight_model::application::command::{Command, CommandOptionType, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::guild::{PartialMember, Permissions, Role};
use twilight_model::id::Id;
use twilight_model::id::marker::{GuildMarker, RoleMarker, UserMarker};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};
use twilight_util::permission_calculator::PermissionCalculator;

use crate::client::event::EventResult;
use crate::command::context::{Context, Visibility};
use crate::command::registry::CommandEntry;
use crate::utility::traits::convert::{AsEmbedAuthor, AsLocale};
use crate::utility::{category, color};

crate::define_entry!("help", CommandType::ChatInput, struct {
    contexts: [InteractionContextType::Guild, InteractionContextType::BotDm],
}, struct {
    command: on_command,
}, struct {});

/// Executes the command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_command<'ap: 'ev, 'ev>(_: &CommandEntry, mut context: Context<'ap, 'ev, &'ev CommandData>) -> EventResult {
    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let mut buffer = String::new();

    writeln!(&mut buffer, "{}", localize!(async(try in locale) category::UI, "help-header").await?)?;

    writeln!(&mut buffer, "### {}:\n", localize!(async(try in locale) category::UI, "help-global").await?)?;

    let commands = context.client().global_commands().await?.model().await?;

    self::write_command_section(&context, locale, commands, &mut buffer).await?;

    if let Some(guild_id) = context.interaction.guild_id {
        writeln!(&mut buffer, "### {}:\n", localize!(async(try in locale) category::UI, "help-guild").await?)?;

        let commands = context.client().guild_commands(guild_id).await?.model().await?;

        self::write_command_section(&context, locale, commands, &mut buffer).await?;
    }

    let title = localize!(async(try in locale) category::UI, "help-title").await?.to_string();
    let footer = localize!(async(try in locale) category::UI, "help-footer").await?.to_string();
    let footer = EmbedFooterBuilder::new(footer.replace("%V", env!("CARGO_PKG_VERSION"))).build();
    let color = if thread_rng().gen_bool(0.5) { color::BRANDING_A } else { color::BRANDING_B }.rgb();
    let author = if let Some(user) = context.api.cache.current_user() {
        user.as_embed_author()?
    } else {
        let user = context.api.client.current_user().await?.model().await?;

        user.as_embed_author()?
    };

    let embed = EmbedBuilder::new().title(title).author(author).color(color).description(buffer).footer(footer);

    context.embed(embed.build(), Visibility::Ephemeral).await?;

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
        writeln!(f, "> *{}*", localize!(async(try in locale) category::UI, "help-missing").await?)?;

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

    let localized_name = localize!(async(try in locale) category::COMMAND, localized_name_key).await?;
    let localized_description = localize!(async(try in locale) category::COMMAND, localized_description_key).await?;
    let has_subcommands = options.iter().any(|option| {
        //
        matches!(option.kind, CommandOptionType::SubCommand | CommandOptionType::SubCommandGroup)
    });

    if has_subcommands { write!(f, "- `/{localized_name}`") } else { write!(f, "- </{name}:{id}>") }?;

    let mut flags = Vec::with_capacity(3);

    if has_subcommands {
        flags.push(localize!(async(try in locale) category::UI, "help-tag-subcommands").await?);
    }
    if contexts.is_some_and(|v| v.iter().any(|v| *v == InteractionContextType::BotDm)) {
        flags.push(localize!(async(try in locale) category::UI, "help-tag-dms").await?);
    }
    if nsfw.unwrap_or(false) {
        flags.push(localize!(async(try in locale) category::UI, "help-tag-nsfw").await?);
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
