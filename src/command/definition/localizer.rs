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

use std::collections::HashSet;

use anyhow::{bail, Result};
use ina_localization::{localize, Locale};
use ina_logging::{info, warn};
use twilight_model::application::command::{
    CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType, CommandType,
};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_util::builder::embed::EmbedBuilder;

use crate::client::event::EventResult;
use crate::command::context::Context;
use crate::command::registry::CommandEntry;
use crate::command::resolver::CommandOptionResolver;
use crate::utility::traits::convert::AsLocale;
use crate::utility::{category, color, fuzzy_contains, Strictness};

crate::define_command!("localizer", CommandType::ChatInput, struct {
    dev_only: true,
}, struct {
    command: on_command,
    autocomplete: on_autocomplete,
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

/// Executes the command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_command<'ap: 'ev, 'ev>(_: &CommandEntry, mut context: Context<'ap, 'ev, &'ev CommandData>) -> EventResult {
    context.defer(true).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localization::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let resolver = CommandOptionResolver::new(context.state);

    if resolver.get_subcommand("reload").is_ok() {
        return self::on_reload_command(&mut context, locale).await;
    }
    if let Ok(resolver) = resolver.get_subcommand("localize") {
        return self::on_localize_command(resolver, locale, context).await;
    }

    bail!("unknown or missing subcommand")
}

/// Executes the reload command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_reload_command<'ap: 'ev, 'ev>(
    context: &mut Context<'ap, 'ev, &CommandData>,
    locale: Option<Locale>,
) -> EventResult {
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

    crate::client::event::pass()
}

/// Executes the localize command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_localize_command<'ap: 'ev, 'ev>(
    resolver: CommandOptionResolver<'ev>,
    locale: Option<Locale>,
    mut context: Context<'ap, 'ev, &'ev CommandData>,
) -> EventResult {
    let category = resolver.get_str("category")?;
    let key = resolver.get_str("key")?;

    let translated = if let Ok(locale_str) = resolver.get_str("locale") {
        let Ok(locale) = locale_str.parse::<Locale>() else {
            let title = localize!(async(try in locale) category::UI, "localize-unknown").await?;

            context.failure(title, Some(format!("`{locale_str}`"))).await?;

            return crate::client::event::pass();
        };

        localize!(async(in locale) category, key).await?
    } else {
        localize!(async(try in locale) category, key).await?
    };

    context.text(translated, true).await?;

    crate::client::event::pass()
}

/// Executes the autocompletion.
///
/// # Errors
///
/// This function will return an error if the autocompletion could not be executed.
async fn on_autocomplete<'ap: 'ev, 'ev>(
    _: &CommandEntry,
    _: Context<'ap, 'ev, &'ev CommandData>,
    option: &'ev str,
    current: &'ev str,
    _: CommandOptionType,
) -> Result<Box<[CommandOptionChoice]>> {
    match option {
        "locale" => self::on_locale_autocomplete(current).await,
        "category" => Ok(self::on_category_autocomplete(current)),
        "key" => Ok(self::on_key_autocomplete(current)),
        option => {
            warn!(async "unknown option '{option}'").await?;

            Ok(Box::new([]))
        }
    }
}

/// Executes the locale autocompletion.
///
/// # Errors
///
/// This function will return an error if the autocompletion could not be executed.
async fn on_locale_autocomplete(current: &str) -> Result<Box<[CommandOptionChoice]>> {
    let mut locales = ina_localization::thread::list().await?.to_vec();

    locales.retain(|l| fuzzy_contains(Strictness::Firm(true), l.to_string(), current));

    let choices = locales.iter().map(|locale| CommandOptionChoice {
        name: locale.to_string(),
        name_localizations: None,
        value: CommandOptionChoiceValue::String(locale.to_string()),
    });

    Ok(choices.collect())
}

/// Executes the category autocompletion.
///
/// # Errors
///
/// This function will return an error if the autocompletion could not be executed.
fn on_category_autocomplete(current: &str) -> Box<[CommandOptionChoice]> {
    let mut categories: HashSet<String> = category::LIST.iter().copied().map(Into::into).collect();

    if !current.is_empty() {
        categories.retain(|c| fuzzy_contains(Strictness::Firm(true), c, current));

        let replaced = current.replace(|c: char| !c.is_alphanumeric(), "-");
        let replaced = replaced.trim_matches(|c: char| !c.is_alphanumeric());

        if !replaced.is_empty() {
            categories.insert(replaced.to_string());
        }
    }

    let choices = categories.into_iter().map(|category| CommandOptionChoice {
        name: category.clone(),
        name_localizations: None,
        value: CommandOptionChoiceValue::String(category),
    });

    choices.collect()
}

/// Executes the key autocompletion.
///
/// # Errors
///
/// This function will return an error if the autocompletion could not be executed.
fn on_key_autocomplete(current: &str) -> Box<[CommandOptionChoice]> {
    if current.is_empty() {
        return Box::new([]);
    }

    let replaced = current.replace(|c: char| !c.is_alphanumeric(), "-");
    let replaced = replaced.trim_matches(|c: char| !c.is_alphanumeric());
    let output = (!replaced.is_empty()).then(|| CommandOptionChoice {
        name: replaced.to_string(),
        name_localizations: None,
        value: CommandOptionChoiceValue::String(replaced.to_string()),
    });

    output.into_iter().collect()
}
