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

use anyhow::{ensure, Result};
use twilight_http::client::InteractionClient;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::{Embed, MessageFlags};
use twilight_model::http::interaction::InteractionResponseType;

use crate::client::api::ApiRef;
use crate::utility::types::modal::ModalData;

/// A command interaction context.
pub type CommandContext<'ar, 'ev> = Context<'ar, 'ev, &'ev CommandData>;
/// A component interaction context.
pub type ComponentContext<'ar, 'ev> = Context<'ar, 'ev, &'ev MessageComponentInteractionData>;
/// A modal interaction context.
pub type ModalContext<'ar, 'ev> = Context<'ar, 'ev, &'ev ModalInteractionData>;

/// An interaction context.
#[derive(Clone, Copy, Debug)]
pub struct Context<'ar: 'ev, 'ev, T>
where
    T: Send,
{
    /// The context's API reference.
    pub api: ApiRef<'ar>,
    /// The interaction reference.
    pub interaction: &'ev Interaction,
    /// The interaction state.
    pub state: T,

    /// Tracks whether this interaction has been deferred.
    is_deferred: bool,
    /// Tracks whether this interaction is marked as ephemeral.
    is_ephemeral: bool,
    /// Tracks whether this interaction has been completed.
    is_completed: bool,
}

impl<'ar: 'ev, 'ev, T> Context<'ar, 'ev, T>
where
    T: Send,
{
    /// Creates a new [`Context<T>`].
    #[inline]
    pub const fn new(api: ApiRef<'ar>, interaction: &'ev Interaction, state: T) -> Self {
        Self { api, interaction, state, is_deferred: false, is_ephemeral: false, is_completed: false }
    }

    /// Returns whether this interaction has been deferred.
    #[inline]
    pub const fn is_deferred(&self) -> bool {
        self.is_deferred
    }

    /// Returns whether this interaction is marked as ephemeral.
    #[inline]
    pub const fn is_ephemeral(&self) -> bool {
        self.is_ephemeral
    }

    /// Returns whether this interaction has been completed.
    #[inline]
    pub const fn is_completed(&self) -> bool {
        self.is_completed
    }

    /// Returns the interaction client of this [`Context<T>`].
    pub fn client(&self) -> InteractionClient {
        self.api.client.interaction(self.interaction.application_id)
    }

    /// Defers the interaction using the given type.
    ///
    /// # Errors
    ///
    /// This function will return an error if `kind` is invalid, or if the context fails to defer the interaction
    /// response, or if this is called on an invalid interaction type.
    async fn defer_any(&mut self, ephemeral: bool, kind: InteractionResponseType) -> Result<()> {
        if self.is_deferred() {
            ensure!(self.is_ephemeral() == ephemeral, "the ephemeral state has already been set");

            return Ok(());
        }

        let flags = if ephemeral { MessageFlags::EPHEMERAL } else { MessageFlags::empty() };

        crate::create_response!(self, struct {
            kind: kind,
            flags: flags,
        })
        .await?;

        self.is_deferred = true;
        self.is_ephemeral = ephemeral;

        Ok(())
    }

    /// Defers the interaction response.
    ///
    /// # Errors
    ///
    /// This function will return an error if the context fails to defer the interaction response, or if this is called
    /// on an invalid interaction type.
    #[inline]
    pub async fn defer(&mut self, ephemeral: bool) -> Result<()> {
        self.defer_any(ephemeral, InteractionResponseType::DeferredChannelMessageWithSource).await
    }

    /// Defers the interaction response.
    ///
    /// # Errors
    ///
    /// This function will return an error if the context fails to defer the interaction response, or if this is called
    /// on an invalid interaction type.
    #[inline]
    pub async fn defer_update(&mut self, ephemeral: bool) -> Result<()> {
        self.defer_any(ephemeral, InteractionResponseType::DeferredUpdateMessage).await
    }

    /// Responds to the interaction with a text message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed or deferred, or if the context fails to
    /// respond to the interaction.
    pub async fn text(&mut self, content: impl Send + Display, ephemeral: bool) -> Result<()> {
        ensure!(!self.is_completed(), "the interaction must not be completed");

        if self.is_deferred() {
            ensure!(self.is_ephemeral() == ephemeral, "the ephemeral state has already been set");

            crate::follow_up_response!(self, struct {
                content: &content.to_string(),
            })
            .await?;
        } else {
            crate::create_response!(self, struct {
                kind: InteractionResponseType::ChannelMessageWithSource,
                content: content.to_string(),
                flags: if ephemeral { MessageFlags::EPHEMERAL } else { MessageFlags::empty() },
            })
            .await?;
        }

        self.is_completed = true;

        Ok(())
    }

    /// Responds to the interaction with an embedded message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed or deferred, or if the context fails to
    /// respond to the interaction.
    pub async fn embed(&mut self, embed: impl Into<Embed> + Send, ephemeral: bool) -> Result<()> {
        ensure!(!self.is_completed(), "the interaction must not be completed");

        if self.is_deferred() {
            ensure!(self.is_ephemeral() == ephemeral, "the ephemeral state has already been set");

            crate::follow_up_response!(self, struct {
                embeds: &[embed.into()],
            })
            .await?;
        } else {
            crate::create_response!(self, struct {
                kind: InteractionResponseType::ChannelMessageWithSource,
                embeds: [embed.into()],
                flags: if ephemeral { MessageFlags::EPHEMERAL } else { MessageFlags::empty() },
            })
            .await?;
        }

        self.is_completed = true;

        Ok(())
    }

    /// Responds to the interaction with a modal pop-up.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction has been completed or deferred, or if the context fails to
    /// respond to the interaction.
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

        self.is_completed = true;

        Ok(())
    }
}

/// Creates an interaction response.
///
/// # Examples
///
/// ```
/// let response = create_response!(context, struct {
///     kind: DeferredChannelMessageWithSource,
/// });
///
/// response.await?;
/// ```
/// ```
/// let response = create_response!(api.client, &interaction, struct {
///     client: api.client,
///     interaction: interaction,
///     kind: DeferredChannelMessageWithSource,
/// });
///
/// response.await?;
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
        $(attachments: $attachments:expr,)?
        $(choices: $choices:expr,)?
        $(components: $components:expr,)?
        $(content: $content:expr,)?
        $(custom_id: $custom_id:expr,)?
        $(embeds: $embeds:expr,)?
        $(flags: $flags:expr,)?
        $(mentions: $mentions:expr,)?
        $(title: $title:expr,)?
        $(tts: $tts:expr,)?
    })) => {
        $client.create_response($id, $token, &::twilight_model::http::interaction::InteractionResponse {
            kind: $kind,
            data: Some(::twilight_util::builder::InteractionResponseDataBuilder::new()
                $(.attachments($attachments))?
                $(.choices($choices))?
                $(.components($components))?
                $(.content($content))?
                $(.custom_id($custom_id))?
                $(.embeds($embeds))?
                $(.flags($flags))?
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
/// let response = follow_up_response! {
///     context: context,
/// };
///
/// response.await?;
/// ```
/// ```
/// let response = follow_up_response! {
///     client: api.client,
///     interaction: interaction,
/// };
///
/// response.await?;
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
            $interaction.token,
            { $($arguments)* }
        ))
    };
    (@new($client:expr, $token:expr, {
        $(attachments: $attachments:expr,)?
        $(components: $components:expr,)?
        $(content: $content:expr,)?
        $(embeds: $embeds:expr,)?
        $(flags: $flags:expr,)?
        $(mentions: $mentions:expr,)?
        $(tts: $tts:expr,)?
    })) => {
        $client.create_followup($token)
            $(.attachments($attachments))?
            $(.components($components))?
            $(.content($content))?
            $(.embeds($embeds))?
            $(.flags($flags))?
            $(.allowed_mentions($mentions))?
            $(.tts($tts))?
    };
}
