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

use std::fmt::Display;

use anyhow::{Result, ensure};
use ina_localizing::locale::Locale;
use twilight_http::client::InteractionClient;
use twilight_model::application::interaction::{Interaction, InteractionType};
use twilight_model::channel::message::{Component, Embed, MessageFlags};
use twilight_model::http::interaction::InteractionResponseType;
use twilight_util::builder::message::{ContainerBuilder, TextDisplayBuilder};

use crate::client::api::ApiRef;
use crate::utility::color;
use crate::utility::traits::convert::AsLocale;
use crate::utility::types::builder::ValidatedBuilder;
use crate::utility::types::modal::ModalData;

/// An interaction context.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct Context<'ar: 'ev, 'ev, T>
where
    T: Send,
{
    /// The context's API reference.
    pub api: ApiRef<'ar>,
    /// The interaction reference.
    pub interaction: &'ev Interaction,
    /// The interaction data.
    pub data: T,

    /// The context's current interaction state.
    state: ContextState,
    /// The context's assigned visibility.
    visibility: Option<Visibility>,
}

impl<'ar: 'ev, 'ev, T> Context<'ar, 'ev, T>
where
    T: Send,
{
    /// Creates a new [`Context<T>`].
    pub const fn new(api: ApiRef<'ar>, interaction: &'ev Interaction, data: T) -> Self {
        Self { api, interaction, data, state: ContextState::Pending, visibility: None }
    }

    /// Returns whether this interaction is pending.
    pub const fn is_pending(&self) -> bool {
        self.state.is_pending()
    }

    /// Returns whether this interaction has been deferred.
    pub const fn is_deferred(&self) -> bool {
        self.state.is_deferred()
    }

    /// Returns whether this interaction has been completed.
    pub const fn is_completed(&self) -> bool {
        self.state.is_completed()
    }

    /// Returns whether this interaction is marked as ephemeral.
    pub const fn is_ephemeral(&self) -> bool {
        matches!(self.visibility, Some(Visibility::Ephemeral))
    }

    /// Returns the interaction client of this [`Context<T>`].
    pub fn client(&self) -> InteractionClient<'_> {
        self.api.client.interaction(self.interaction.application_id)
    }

    /// Defers the interaction using the given type.
    ///
    /// # Errors
    ///
    /// This function will return an error if `kind` is invalid, or if the context fails to defer the interaction
    /// response, or if this is called on an invalid interaction type.
    async fn defer_any(&mut self, visibility: Visibility, kind: InteractionResponseType) -> Result<()> {
        if let Some(preset) = self.visibility {
            ensure!(preset == visibility, "the response visibility has already been set");
        }
        if self.state.is_deferred() {
            return Ok(());
        }

        let flags = if visibility.is_ephemeral() { MessageFlags::EPHEMERAL } else { MessageFlags::empty() };

        crate::create_response!(self, struct {
            kind: kind,
            flags: flags,
        })
        .await?;

        self.state = ContextState::Deferred;
        self.visibility = Some(visibility);

        Ok(())
    }

    /// Defers the interaction response.
    ///
    /// # Errors
    ///
    /// This function will return an error if the context fails to defer the interaction response, or if this is called
    /// on an invalid interaction type.
    pub async fn defer(&mut self, visibility: Visibility) -> Result<()> {
        self.defer_any(visibility, InteractionResponseType::DeferredChannelMessageWithSource).await
    }

    /// Defers the interaction response.
    ///
    /// # Errors
    ///
    /// This function will return an error if the context fails to defer the interaction response, or if this is called
    /// on an invalid interaction type.
    pub async fn defer_update(&mut self, visibility: Visibility) -> Result<()> {
        ensure!(
            matches!(self.interaction.kind, InteractionType::MessageComponent | InteractionType::ModalSubmit),
            "invalid interaction type"
        );

        self.defer_any(visibility, InteractionResponseType::DeferredUpdateMessage).await
    }

    /// Set [`Self`] as being [`ContextState::Completed`], marking the end of an interaction.
    pub const fn complete(&mut self) {
        self.state = ContextState::Completed;
    }

    /// Responds to the interaction with a text message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed, or if the context fails to respond to
    /// the interaction.
    pub async fn text(&mut self, content: impl Send + Display, visibility: Visibility) -> Result<()> {
        ensure!(!self.is_completed(), "the interaction must not be completed");

        if let Some(assigned) = self.visibility {
            ensure!(assigned == visibility, "the response visibility has already been set");
        }

        match self.state {
            ContextState::Pending => {
                crate::create_response!(self, struct {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    flags: if visibility.is_ephemeral() { MessageFlags::EPHEMERAL } else { MessageFlags::empty() },
                    content: content.to_string(),
                })
                .await?;
            }
            ContextState::Deferred => {
                crate::follow_up_response!(self, struct {
                    content: &content.to_string(),
                })
                .await?;
            }
            ContextState::Completed => unreachable!("the interaction must not be completed"),
        }

        self.state = ContextState::Completed;
        self.visibility = Some(visibility);

        Ok(())
    }

    /// Responds to the interaction with an embedded message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed, or if the context fails to respond to
    /// the interaction.
    pub async fn embed(&mut self, embed: impl Into<Embed> + Send, visibility: Visibility) -> Result<()> {
        ensure!(!self.is_completed(), "the interaction must not be completed");

        if let Some(assigned) = self.visibility {
            ensure!(assigned == visibility, "the response visibility has already been set");
        }

        match self.state {
            ContextState::Pending => {
                crate::create_response!(self, struct {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    flags: if visibility.is_ephemeral() { MessageFlags::EPHEMERAL } else { MessageFlags::empty() },
                    embeds: [embed.into()],
                })
                .await?;
            }
            ContextState::Deferred => {
                crate::follow_up_response!(self, struct {
                    embeds: &[embed.into()],
                })
                .await?;
            }
            ContextState::Completed => unreachable!("the interaction must not be completed"),
        }

        self.state = ContextState::Completed;
        self.visibility = Some(visibility);

        Ok(())
    }

    /// Responds to the interaction with a component-based message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed, or if the context fails to respond to
    /// the interaction.
    pub async fn components<C, I>(&mut self, components: I, visibility: Visibility) -> Result<()>
    where
        C: Into<Component>,
        I: IntoIterator<Item = C>,
    {
        ensure!(!self.is_completed(), "the interaction must not be completed");

        if let Some(assigned) = self.visibility {
            ensure!(assigned == visibility, "the response visibility has already been set");
        }

        match self.state {
            ContextState::Pending => {
                crate::create_response!(self, struct {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    flags: MessageFlags::IS_COMPONENTS_V2 | (
                        if visibility.is_ephemeral() { MessageFlags::EPHEMERAL } else { MessageFlags::empty() }
                    ),
                    components: components.into_iter().map(Into::into),
                })
                .await?;
            }
            ContextState::Deferred => {
                crate::follow_up_response!(self, struct {
                    flags: MessageFlags::IS_COMPONENTS_V2,
                    components: &(components.into_iter().map(Into::into).collect::<Box<[_]>>()),
                })
                .await?;
            }
            ContextState::Completed => unreachable!("the interaction must not be completed"),
        }

        self.state = ContextState::Completed;
        self.visibility = Some(visibility);

        Ok(())
    }

    /// Responds to the interaction with a modal pop-up.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed, or if the context fails to respond to
    /// the interaction.
    pub async fn modal(&mut self, ModalData { custom_id, title, components }: ModalData) -> Result<()> {
        ensure!(!self.is_completed(), "the interaction must not be completed");
        ensure!(!self.is_deferred(), "the interaction must not be deferred");

        crate::create_response!(self, struct {
            kind: InteractionResponseType::Modal,
            components: components,
            custom_id: custom_id,
            title: title,
        })
        .await?;

        self.state = ContextState::Completed;

        Ok(())
    }

    /// Finishes an interaction with an embedded message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed, or if the context fails to respond to
    /// the interaction.
    async fn finish_with_message<N, D>(&mut self, color: u32, title: N, description: Option<D>) -> Result<()>
    where
        N: Display + Send,
        D: Display + Send,
    {
        let mut container = ContainerBuilder::new()
            .accent_color(Some(color))
            .component(TextDisplayBuilder::new(format!("### {title}")).try_build()?);

        if let Some(description) = description {
            container = container.component(TextDisplayBuilder::new(description.to_string()).try_build()?);
        }

        self.components([container.try_build()?], Visibility::Ephemeral).await
    }

    /// Finishes an interaction with an embedded success message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed, or if the context fails to respond to
    /// the interaction.
    pub async fn success_message<N, D>(&mut self, title: N, description: Option<D>) -> Result<()>
    where
        N: Display + Send,
        D: Display + Send,
    {
        self.finish_with_message(color::SUCCESS.rgb(), title, description).await
    }

    /// Finishes an interaction with an embedded completion message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed, or if the context fails to respond to
    /// the interaction.
    pub async fn complete_message<N, D>(&mut self, title: N, description: Option<D>) -> Result<()>
    where
        N: Display + Send,
        D: Display + Send,
    {
        self.finish_with_message(color::BRANDING.rgb(), title, description).await
    }

    /// Finishes an interaction with an embedded warning message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed, or if the context fails to respond to
    /// the interaction.
    pub async fn warning_message<N, D>(&mut self, title: N, description: Option<D>) -> Result<()>
    where
        N: Display + Send,
        D: Display + Send,
    {
        self.finish_with_message(color::BACKDROP.rgb(), title, description).await
    }

    /// Finishes an interaction with an embedded failure message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed, or if the context fails to respond to
    /// the interaction.
    pub async fn failure_message<N, D>(&mut self, title: N, description: Option<D>) -> Result<()>
    where
        N: Display + Send,
        D: Display + Send,
    {
        self.finish_with_message(color::FAILURE.rgb(), title, description).await
    }
}

impl<'ar: 'ev, 'ev, T> AsLocale for Context<'ar, 'ev, T>
where
    T: Send,
{
    type Error = ina_localizing::Error;

    /// Fallibly converts this value into a translation locale.
    ///
    /// This is resolved in the following priority order:
    ///     1. Interaction locale
    ///     2. User locale
    ///     3. Guild locale
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be converted.
    fn as_locale(&self) -> Result<Locale, Self::Error> {
        // Prefer the interaction-specified locale.
        self.interaction
            .locale
            .as_deref()
            // Fall back to the author's locale.
            .or_else(|| self.interaction.author().and_then(|u| u.locale.as_deref()))
            // Fall back to the guild's locale.
            .or(self.interaction.guild_locale.as_deref())
            // Attempt to parse it into a valid locale value.
            .map(str::parse).transpose()?
            // Or fail and say that it's missing.
            .ok_or(ina_localizing::Error::MissingLocale)
    }
}

/// Describes the user visibility of a response.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Visibility {
    /// The response is visible to all users.
    #[default]
    Visible,
    /// The response is only visible to a specific user.
    Ephemeral,
}

impl Visibility {
    /// Returns `true` if the visibility is [`Visible`].
    ///
    /// [`Visible`]: Visibility::Visible
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        matches!(self, Self::Visible)
    }

    /// Returns `true` if the visibility is [`Ephemeral`].
    ///
    /// [`Ephemeral`]: Visibility::Ephemeral
    #[must_use]
    pub const fn is_ephemeral(&self) -> bool {
        matches!(self, Self::Ephemeral)
    }
}

/// Describes the current state of a context's interaction.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextState {
    /// The interaction is pending.
    #[default]
    Pending,
    /// The interaction has been deferred.
    Deferred,
    /// The interaction has been completed.
    Completed,
}

impl ContextState {
    /// Returns `true` if the context state is [`Pending`].
    ///
    /// [`Pending`]: ContextState::Pending
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns `true` if the context state is [`Deferred`].
    ///
    /// [`Deferred`]: ContextState::Deferred
    #[must_use]
    pub const fn is_deferred(&self) -> bool {
        matches!(self, Self::Deferred)
    }

    /// Returns `true` if the context state is [`Completed`].
    ///
    /// [`Completed`]: ContextState::Completed
    #[must_use]
    pub const fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }
}

/// Creates an interaction response.
///
/// # Examples
///
/// ```
/// // Defer a message.
/// create_response!(context, struct {
///     kind: InteractionResponseType::DeferredChannelMessageWithSource,
/// })
/// .await?;
/// ```
/// ```
/// // Respond with an embed.
/// let embed = EmbedBuilder::new().title("An embedded message!");
///
/// create_response!(api.client, &interaction, struct {
///     kind: InteractionResponseType::ChannelMessageWithSource,
///     embeds: [embed.build()]
/// })
/// .await?;
/// ```
#[macro_export]
macro_rules! create_response {
    ($context:expr, $kind:expr) => {
        $crate::create_response!($context, struct { kind = $kind, })
    };
    ($client:expr, $interaction:expr, $kind:expr) => {
        $crate::create_response!($client, $interaction, struct { kind = $kind, })
    };
    ($context:expr, struct { $($arguments:tt)+ }) => {
        $crate::create_response!(@new(
            $context.client(),
            $context.interaction.id,
            &$context.interaction.token,
            { $($arguments)+ }
        ))
    };
    ($client:expr, $interaction:expr, struct { $($arguments:tt)+ }) => {
        $crate::create_response!(@new(
            $client.interaction($interaction.application_id),
            $interaction.id,
            &$interaction.token,
            { $($arguments)+ }
        ))
    };
    (@new($client:expr, $id:expr, $token:expr, {
        kind: $kind:expr,
        $(flags: $flags:expr,)?
        $(attachments: $attachments:expr,)?
        $(choices: $choices:expr,)?
        $(components: $components:expr,)?
        $(content: $content:expr,)?
        $(custom_id: $custom_id:expr,)?
        $(embeds: $embeds:expr,)?
        $(mentions: $mentions:expr,)?
        $(title: $title:expr,)?
        $(tts: $tts:expr,)?
    })) => {
        $client.create_response($id, $token, &::twilight_model::http::interaction::InteractionResponse {
            kind: $kind,
            data: Some(::twilight_util::builder::InteractionResponseDataBuilder::new()
                $(.flags($flags))?
                $(.attachments($attachments))?
                $(.choices($choices))?
                $(.components($components))?
                $(.content($content))?
                $(.custom_id($custom_id))?
                $(.embeds($embeds))?
                $(.allowed_mentions($mentions))?
                $(.title($title))?
                $(.tts($tts))?
                .build()
            ),
        })
    };
}

/// Follows-up an interaction response.
///
/// # Examples
///
/// ```
/// /// An empty follow-up.
/// follow_up_response!(context, struct {}).await?;
/// ```
/// ```
/// /// Follow up with an embed.
/// follow_up_response!(api.client, interaction, struct {
///     embeds: &[embed.build()],
/// })
/// .await?;
/// ```
#[macro_export]
macro_rules! follow_up_response {
    ($context:expr) => {
        $crate::follow_up_response!($context, struct {})
    };
    ($client:expr, $interaction:expr) => {
        $crate::follow_up_response!($client, $interaction, struct {})
    };
    ($context:expr, struct { $($arguments:tt)* }) => {
        $crate::follow_up_response!(@new(
            $context.client(),
            &$context.interaction.token,
            { $($arguments)* }
        ))
    };
    ($client:expr, $interaction:expr, struct { $($arguments:tt)* }) => {
        $crate::follow_up_response!(@new(
            $client.interaction($interaction.application_id),
            &$interaction.token,
            { $($arguments)* }
        ))
    };
    (@new($client:expr, $token:expr, {
        $(flags: $flags:expr,)?
        $(attachments: $attachments:expr,)?
        $(components: $components:expr,)?
        $(content: $content:expr,)?
        $(embeds: $embeds:expr,)?
        $(mentions: $mentions:expr,)?
        $(tts: $tts:expr,)?
    })) => {
        $client.create_followup($token)
            $(.flags($flags))?
            $(.attachments($attachments))?
            $(.components($components))?
            $(.content($content))?
            $(.embeds($embeds))?
            $(.allowed_mentions($mentions))?
            $(.tts($tts))?
    };
}
