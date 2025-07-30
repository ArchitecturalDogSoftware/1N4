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

//! Definitions for button components for the `/help` command response that respond with files when
//! pressed.
//!
//! These files are embedded into the binary at build time, but will also check `res/attachments/`
//! for the file (specifically, any file named as it is sent in the response message) when called
//! at runtime. This allows instance administrators to change the contents without compiling their
//! own binary.

/// Creates a module containing a generator function and a callback for a button component that
/// responds with a file.
macro_rules! attachment_button {
    (
        $button_id:ident,
        $localization_key:expr,
        $icon:expr,
        $embedded_input_dir:expr,
        $input_file_name:expr,
        $output_file_name:expr,
    ) => {
            pub mod $button_id {
                #![doc = ::std::concat!("Definitions for a generator ([`button`]) and a component callback ([`on_component`]) for `", ::std::stringify!($button_id), "`.")]

                #[doc = ::std::concat!("Creates a button (ID `", ::std::stringify!($button_id), "`), which responds with an attachment.")]
                #[doc = "\nSee [`on_component`] for more information on the response."]
                #[doc = "\n# Errors\n"]
                #[doc = "This function will return an error if the button is constructed incorrectly or if localization fails."]
                pub async fn button(
                    locale: Option<::ina_localizing::locale::Locale>,
                    command_name: &'static ::std::primitive::str
                ) -> ::anyhow::Result<::twilight_model::channel::message::component::Button> {
                    let button = $crate::utility::types::builder::ButtonBuilder::new(
                        ::twilight_model::channel::message::component::ButtonStyle::Secondary
                    )
                    .label(
                        ::ina_localizing::localize!(
                            async(try in locale) $crate::utility::category::UI, $localization_key
                        ).await?.to_string()
                    )?
                    .emoji(::twilight_model::channel::message::EmojiReactionType::Unicode { name: $icon.to_string() })?
                    .custom_id($crate::utility::types::custom_id::CustomId::new(command_name, ::std::stringify!($button_id))?)?
                    .build();

                    Ok(button)
                }

                #[doc = ::std::concat!("Executes the `", ::std::stringify!($button_id), "` component, sending either a copy of the given file from `res/attachments/` or an embedded copy.")]
                #[doc = "\n# Errors\n"]
                #[doc = "This function will return an error if the component could not be executed."]
                pub async fn on_component<'ap: 'ev, 'ev>(
                    _: &$crate::command::registry::CommandEntry,
                    mut context: $crate::command::context::Context<
                        'ap,
                        'ev,
                        &'ev ::twilight_model::application::interaction::message_component::MessageComponentInteractionData
                    >,
                    _: $crate::utility::types::custom_id::CustomId,
                ) -> $crate::client::event::EventResult {
                    use ::std::io::Read;

                    const OUTPUT_FILE_NAME: &::std::primitive::str = $output_file_name;
                    const FILE_CONTENT: &[::std::primitive::u8] = include_bytes!(
                        ::std::concat!($embedded_input_dir, "/", $input_file_name)
                    );
                    // Almost completely arbitrary. Can be anything, so long as it is unique within the same message.
                    const FILE_ID: ::std::primitive::u64 = 0;

                    // TO-DO: this is better as a thread settings call.
                    let resources_dir = ::std::env::current_dir()
                        .map_or_else(|_| ::std::path::PathBuf::from("./res/attachments"), |v| v.join("res/attachments"));

                    let mut buf = ::std::vec::Vec::new();
                    let file_content = ::std::fs::File::open(resources_dir.join($output_file_name))
                        .and_then(|mut f| f.read_to_end(&mut buf).map(|_| buf.as_slice()))
                        .unwrap_or(FILE_CONTENT);

                    context.defer($crate::command::context::Visibility::Ephemeral).await?;

                    let license_file = ::twilight_model::http::attachment::Attachment::from_bytes(
                        OUTPUT_FILE_NAME.to_string(),
                        file_content.to_vec(),
                        FILE_ID,
                    );

                    $crate::follow_up_response!(context, struct {
                        attachments: &[license_file],
                    })
                    .await?;
                    context.complete();

                    $crate::client::event::pass()
                }
        }
    };

    ($button_id:ident, $localization_key:expr, $icon:expr, $embedded_input_dir:expr, $input_file_name:expr,) => {
        attachment_button!(
            $button_id,
            $localization_key,
            $icon,
            $embedded_input_dir,
            $input_file_name,
            $input_file_name,
        );
    }
}

attachment_button!(licenses, "help-button-licenses", "📃", env!("OUT_DIR"), "licenses.md",);
attachment_button!(
    privacy_policy,
    "help-button-privacy-policy",
    "🔐",
    concat!(env!("CARGO_MANIFEST_DIR"), "/docs"),
    "PRIVACY_POLICY.md",
);
attachment_button!(
    security_policy,
    "help-button-security-policy",
    "📢",
    concat!(env!("CARGO_MANIFEST_DIR"), "/docs"),
    "SECURITY.md",
    "SECURITY_POLICY.md",
);
