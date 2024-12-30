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

use anyhow::Result;
use ina_localizing::locale::Locale;
use ina_localizing::localize;
use ina_logging::{info, warn};
use twilight_model::application::command::{
    CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType, CommandType,
};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::application::interaction::application_command::CommandData;

use crate::client::event::EventResult;
use crate::command::context::{Context, Visibility};
use crate::command::registry::CommandEntry;
use crate::command::resolver::CommandOptionResolver;
use crate::utility::category;
use crate::utility::search::{Strictness, fuzzy_contains};
use crate::utility::traits::convert::AsLocale;

crate::define_entry!("localizer", CommandType::ChatInput, struct {
    dev_only: true,
    contexts: [InteractionContextType::Guild],
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

crate::define_commands! {
    self => {
        reload => on_reload_command;
        localize => on_localize_command;
    }
}

/// Executes the reload command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_reload_command<'ap: 'ev, 'ev>(
    _: &CommandEntry,
    mut context: Context<'ap, 'ev, &CommandData>,
    _: CommandOptionResolver<'ev>,
) -> EventResult {
    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    info!(async "reloading localization thread").await?;

    // Do we want to clear here? It may cause concurrent commands to fail to localize.
    ina_localizing::thread::clear(None::<[_; 0]>).await?;

    let loaded_locales = ina_localizing::thread::load(None::<[_; 0]>).await?;

    info!(async "loaded {loaded_locales} localization locales").await?;

    let title = localize!(async(try in locale) category::UI, "localizer-reloaded").await?;
    let locales = localize!(async(try in locale) category::UI, "localizer-locales").await?;

    let list = ina_localizing::thread::list().await?;
    let list = list.iter().map(|l| format!("`{l}`"));
    let locales = format!("{locales}:\n> {}", list.collect::<Box<[_]>>().join(", "));

    context.success(title, Some(locales)).await?;

    crate::client::event::pass()
}

/// Executes the localize command.
///
/// # Errors
///
/// This function will return an error if the command could not be executed.
async fn on_localize_command<'ap: 'ev, 'ev>(
    _: &CommandEntry,
    mut context: Context<'ap, 'ev, &'ev CommandData>,
    resolver: CommandOptionResolver<'ev>,
) -> EventResult {
    context.defer(Visibility::Ephemeral).await?;

    let locale = match context.as_locale() {
        Ok(locale) => Some(locale),
        Err(ina_localizing::Error::MissingLocale) => None,
        Err(error) => return Err(error.into()),
    };

    let category = resolver.string("category")?;
    let key = resolver.string("key")?;

    let translated = if let Ok(locale_str) = resolver.string("locale") {
        let Ok(locale) = locale_str.parse::<Locale>() else {
            let title = localize!(async(try in locale) category::UI, "localize-unknown").await?;

            context.failure(title, Some(format!("`{locale_str}`"))).await?;

            return crate::client::event::pass();
        };

        localize!(async(in locale) category, key).await?
    } else {
        localize!(async(try in locale) category, key).await?
    };

    context.text(format!("`{category}::{key}`\n\n{translated}"), Visibility::Ephemeral).await?;

    crate::client::event::pass()
}

/// Executes the auto-completion.
///
/// # Errors
///
/// This function will return an error if the auto-completion could not be executed.
async fn on_autocomplete<'ap: 'ev, 'ev>(
    _: &CommandEntry,
    context: Context<'ap, 'ev, &'ev CommandData>,
    option: &'ev str,
    current: &'ev str,
    _: CommandOptionType,
) -> Result<Box<[CommandOptionChoice]>> {
    match option {
        "locale" => self::on_locale_autocomplete(current).await,
        "category" => Ok(self::on_category_autocomplete(current)),
        "key" => {
            let resolver = CommandOptionResolver::new(context.data);
            let resolver = resolver.subcommand("localize")?;

            let locale = resolver.string("locale").ok().and_then(|v| v.parse().ok());
            let category = resolver.string("category").ok().map(Box::from);

            self::on_key_autocomplete(locale, category, current).await
        }
        option => {
            warn!(async "unknown option '{option}'").await?;

            Ok(Box::new([]))
        }
    }
}

/// Executes the locale auto-completion.
///
/// # Errors
///
/// This function will return an error if the auto-completion could not be executed.
async fn on_locale_autocomplete(current: &str) -> Result<Box<[CommandOptionChoice]>> {
    let mut locales = ina_localizing::thread::list().await?.to_vec();

    locales.retain(|l| fuzzy_contains(Strictness::Firm { ignore_casing: true }, l.to_string(), current));

    let choices = locales.iter().map(|locale| CommandOptionChoice {
        name: locale.to_string(),
        name_localizations: None,
        value: CommandOptionChoiceValue::String(locale.to_string()),
    });

    Ok(choices.collect())
}

/// Executes the category auto-completion.
///
/// # Errors
///
/// This function will return an error if the auto-completion could not be executed.
fn on_category_autocomplete(current: &str) -> Box<[CommandOptionChoice]> {
    let mut categories: HashSet<String> = category::LIST.iter().copied().map(Into::into).collect();

    if !current.is_empty() {
        categories.retain(|c| fuzzy_contains(Strictness::Firm { ignore_casing: true }, c, current));

        if categories.is_empty() {
            let replaced = current.replace(|c: char| !c.is_alphanumeric(), "-");
            let replaced = replaced.trim_matches(|c: char| !c.is_alphanumeric());

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

/// Executes the key auto-completion.
///
/// # Errors
///
/// This function will return an error if the auto-completion could not be executed.
async fn on_key_autocomplete(
    locale: Option<Locale>,
    category: Option<Box<str>>,
    current: &str,
) -> Result<Box<[CommandOptionChoice]>> {
    let mut keys = if let Some(category) = category {
        ina_localizing::thread::keys(locale, category).await?.into_vec()
    } else {
        vec![]
    };

    if !current.is_empty() {
        keys.retain(|c| fuzzy_contains(Strictness::Firm { ignore_casing: true }, c, current));

        if keys.is_empty() {
            let replaced = current.replace(|c: char| !c.is_alphanumeric(), "-");
            let replaced = replaced.trim_matches(|c: char| !c.is_alphanumeric());

            keys.push(replaced.to_string().into());
        }
    }

    let choices = keys.into_iter().map(|key| CommandOptionChoice {
        name: key.to_string(),
        name_localizations: None,
        value: CommandOptionChoiceValue::String(key.to_string()),
    });

    Ok(choices.collect())
}
