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
use ina_localization::{localize, Locale};
use ina_logging::{info, warn};
use twilight_model::application::command::{
    CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType, CommandType,
};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_util::builder::embed::EmbedBuilder;

use crate::command::context::Context;
use crate::command::resolver::CommandOptionResolver;
use crate::utility::traits::convert::AsLocale;
use crate::utility::{category, color};

crate::define_command!("localizer", CommandType::ChatInput, struct {
    dev_only: true,
}, struct {
    command_callback: on_command,
    autocomplete_callback: on_autocomplete,
}, struct {
    reload: SubCommand {},
    localize: SubCommand {
        category: String {
            required: true,
            autocomplete: true,
        },
        key: String {
            required: true,
            autocomplete: true,
        },
        locale: String {
            autocomplete: true,
        },
    },
});

/// Returns whether `current` is contained within `source`, ignoring casing and symbols.
fn fuzzy_contains(source: &str, current: &str) -> bool {
    let source = source.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
    let current = current.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");

    source.contains(&current)
}

/// Executes the command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_command<'ap: 'ev, 'ev>(mut context: Context<'ap, 'ev, &'ev CommandData>) -> Result<bool> {
    context.defer(true).await?;

    let resolver = CommandOptionResolver::new(context.state);
    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localization::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    if resolver.get_subcommand("reload").is_ok() {
        info!(async "reloading localization thread").await?;

        // Do we want to clear here? It may cause concurrent commands to fail to localize.
        ina_localization::thread::clear().await?;

        let loaded_locales = ina_localization::thread::load(None).await?;

        info!(async "loaded {loaded_locales} localization locales").await?;

        let title = localize!(async(try in locale) category::UI, "localizer-reloaded").await?;
        let locales = localize!(async(try in locale) category::UI, "localizer-locales").await?;

        let list = ina_localization::thread::list().await?;
        let list = list.iter().map(|l| format!("`{l}`"));
        let locales = format!("{locales}:\n> {}", list.collect::<Box<[_]>>().join(", "));

        let embed = EmbedBuilder::new().title(title).color(color::SUCCESS).description(locales);

        context.embed(embed.build(), true).await?;

        return Ok(false);
    }

    if let Ok(resolver) = resolver.get_subcommand("localize") {
        let category = resolver.get_str("category")?;
        let key = resolver.get_str("key")?;

        let translated = if let Ok(locale_str) = resolver.get_str("locale") {
            let Ok(locale) = locale_str.parse::<Locale>() else {
                // Maybe make this failure pattern into a macro/function?
                let title = localize!(async(try in locale) category::UI, "localize-unknown").await?;
                let embed = EmbedBuilder::new().title(format!("{title}: '{locale_str}'")).color(color::FAILURE);

                context.embed(embed.build(), true).await?;

                return Ok(false);
            };

            localize!(async(in locale) category, key).await?
        } else {
            localize!(async(try in locale) category, key).await?
        };

        context.text(translated, true).await?;

        return Ok(false);
    }

    bail!("unknown or missing subcommand")
}

/// Executes the autocompletion.
///
/// # Errors
///
/// This function will return an error if the autocompletion could not be executed.
async fn on_autocomplete<'ap: 'ev, 'ev>(
    _: Context<'ap, 'ev, &'ev CommandData>,
    option: &'ev str,
    current: &'ev str,
    _: CommandOptionType,
) -> Result<Box<[CommandOptionChoice]>> {
    match option {
        "locale" => {
            let mut locales = ina_localization::thread::list().await?.to_vec();

            locales.retain(|l| self::fuzzy_contains(&l.to_string(), current));

            let choices = locales.iter().map(|l| {
                let value = CommandOptionChoiceValue::String(l.to_string());

                CommandOptionChoice { name: l.to_string(), name_localizations: None, value }
            });

            Ok(choices.collect())
        }
        "category" => {
            let mut categories: Vec<String> = category::LIST.iter().copied().map(Into::into).collect();

            if !current.is_empty() {
                categories.retain(|c| self::fuzzy_contains(c, current));

                let replaced = current.replace(|c: char| !c.is_alphanumeric(), "-");

                categories.push(replaced);
            }

            let choices = categories.into_iter().map(|name| {
                let value = CommandOptionChoiceValue::String(name.clone());

                CommandOptionChoice { name, name_localizations: None, value }
            });

            Ok(choices.collect())
        }
        "key" if current.is_empty() => Ok(Box::new([])),
        "key" => {
            let replaced = current.replace(|c: char| !c.is_alphanumeric(), "-");

            Ok(Box::new([CommandOptionChoice {
                name: replaced.clone(),
                name_localizations: None,
                value: CommandOptionChoiceValue::String(replaced),
            }]))
        }
        option => {
            warn!(async "unknown option '{option}'").await?;

            Ok(Box::default())
        }
    }
}
