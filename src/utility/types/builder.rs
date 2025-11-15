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

use std::error::Error;

use twilight_model::channel::message::Component;
use twilight_model::channel::message::component::{
    ActionRow, Button, Container, FileDisplay, FileUpload, Label, MediaGallery, MediaGalleryItem, Section, SelectMenu,
    SelectMenuOption, Separator, TextDisplay, TextInput, TextInputStyle, Thumbnail, UnfurledMediaItem,
};
use twilight_util::builder::message::{
    ActionRowBuilder, ButtonBuilder, ContainerBuilder, FileDisplayBuilder, FileUploadBuilder, LabelBuilder,
    SectionBuilder, SelectMenuBuilder, SelectMenuOptionBuilder, SeparatorBuilder, TextDisplayBuilder, ThumbnailBuilder,
};
use twilight_validate::component::ComponentValidationError;

use crate::utility::traits::extension::UnfurledMediaItemExt;

/// A builder that automatically validates the inner type when completed.
pub trait ValidatedBuilder {
    /// The output type.
    type Output: Sized;
    /// The error type.
    type Error: Error + Sized;

    /// Validates that the constructed value is considered valid.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value is invalid.
    fn validate(inner: &Self::Output) -> Result<(), Self::Error>;

    /// Builds the value, returning it if it is valid.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value is invalid.
    fn try_build(self) -> Result<Self::Output, Self::Error>;
}

/// Implements [`ValidatedBuilder`] for various types.
///
/// # Examples
///
/// ```
/// define_validated_builders! {
///     ContainerBuilder => Container : twilight_validate::component::container;
/// }
/// ```
macro_rules! define_validated_builders {
    ($($type:path => $output:path : $function:path $([ $($args:expr), +$(,)? ])?;)*) => {
        $(
            impl ValidatedBuilder for $type {
                type Output = $output;
                type Error = ComponentValidationError;

                #[inline]
                fn validate(inner: &Self::Output) -> Result<(), Self::Error> {
                    $function(inner $(, $($args),+)?)
                }

                fn try_build(self) -> Result<Self::Output, Self::Error> {
                    let inner = self.build();

                    <Self as ValidatedBuilder>::validate(&inner).map(|()| inner)
                }
            }
        )*
    };
}

define_validated_builders! {
    ActionRowBuilder => ActionRow : twilight_validate::component::action_row [true];
    ButtonBuilder => Button : twilight_validate::component::button;
    FileDisplayBuilder => FileDisplay : never_validate;
    FileUploadBuilder => FileUpload : twilight_validate::component::file_upload;
    ContainerBuilder => Container : twilight_validate::component::container;
    LabelBuilder => Label : twilight_validate::component::label;
    MediaGalleryBuilder => MediaGallery : twilight_validate::component::media_gallery;
    MediaGalleryItemBuilder => MediaGalleryItem : twilight_validate::component::media_gallery_item;
    SectionBuilder => Section : twilight_validate::component::section;
    SelectMenuBuilder => SelectMenu : twilight_validate::component::select_menu [false];
    SelectMenuOptionBuilder => SelectMenuOption : never_validate;
    SeparatorBuilder => Separator : never_validate;
    TextDisplayBuilder => TextDisplay : twilight_validate::component::text_display;
    TextInputBuilder => TextInput : twilight_validate::component::text_input [false];
    ThumbnailBuilder => Thumbnail : twilight_validate::component::thumbnail;
}

/// Always considers the given component valid.
///
/// This function can be removed by passing the `INA_COMPONENT_VALIDATION=strict` environment variable during
/// compilation.
///
/// # Errors
///
/// This function will never return an error.
#[cfg(ina_component_validation = "relaxed")]
#[inline]
pub const fn never_validate<T, E>(_: &T) -> Result<(), E> {
    Ok(())
}

/// This function fails with a compile error.
///
/// # Errors
///
/// This function will never return an error.
#[cfg(ina_component_validation = "strict")]
#[inline]
pub const fn never_validate<T, E>(_: &T) -> Result<(), E> {
    compile_error!("the component builder must be validated");
}

/// Builds a [`MediaGallery`].
#[must_use = "builders must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaGalleryBuilder(MediaGallery);

impl MediaGalleryBuilder {
    /// Creates a new [`MediaGalleryBuilder`].
    pub const fn new() -> Self {
        Self(MediaGallery { id: None, items: Vec::new() })
    }

    /// Sets the media gallery's numeric identifier.
    pub const fn id(mut self, id: i32) -> Self {
        self.0.id = Some(id);
        self
    }

    /// Adds an item to the media gallery.
    ///
    /// # Errors
    ///
    /// This function will return an error if too many items have been added.
    pub fn item(mut self, item: impl Into<MediaGalleryItem>) -> Self {
        self.0.items.push(item.into());
        self
    }

    /// Builds the completed text input.
    #[must_use]
    pub fn build(self) -> MediaGallery {
        self.0
    }
}

impl Default for MediaGalleryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl From<MediaGalleryBuilder> for MediaGallery {
    fn from(value: MediaGalleryBuilder) -> Self {
        value.build()
    }
}

impl From<MediaGalleryBuilder> for Component {
    fn from(value: MediaGalleryBuilder) -> Self {
        Self::MediaGallery(value.build())
    }
}
/// Builds a [`MediaGalleryItem`].
#[must_use = "builders must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaGalleryItemBuilder(MediaGalleryItem);

impl MediaGalleryItemBuilder {
    /// Creates a new [`MediaGalleryItemBuilder`].
    pub fn new(media: impl Into<UnfurledMediaItem>) -> Self {
        Self(MediaGalleryItem { media: media.into(), description: None, spoiler: None })
    }

    /// Creates a new [`MediaGalleryItemBuilder`] using the given URL.
    pub fn url(url: impl Into<String>) -> Self {
        Self::new(UnfurledMediaItem::url(url))
    }

    /// Sets the media gallery item's description.
    ///
    /// # Errors
    ///
    /// This function will return an error if the given description is too long.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.0.description = Some(description.into());
        self
    }

    /// Sets whether the media gallery item is spoilered.
    pub const fn spoiler(mut self, spoiler: bool) -> Self {
        self.0.spoiler = Some(spoiler);
        self
    }

    /// Builds the completed text input.
    #[must_use]
    pub fn build(self) -> MediaGalleryItem {
        self.0
    }
}

impl From<MediaGalleryItemBuilder> for MediaGalleryItem {
    fn from(value: MediaGalleryItemBuilder) -> Self {
        value.build()
    }
}

/// Builds a [`TextInput`].
#[must_use = "builders must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInputBuilder(TextInput);

impl TextInputBuilder {
    /// Creates a new [`TextInputBuilder`].
    ///
    /// # Errors
    ///
    /// This function will return an error if a value exceeds the character limit.
    #[expect(deprecated, reason = "we still need to set the field, even if it's just to `None`")]
    pub fn new(custom_id: impl Into<String>, style: TextInputStyle) -> Self {
        Self(TextInput {
            id: None,
            custom_id: custom_id.into(),
            label: None,
            max_length: None,
            min_length: None,
            placeholder: None,
            required: None,
            style,
            value: None,
        })
    }

    /// Sets the text input's numeric identifier.
    pub const fn id(mut self, id: i32) -> Self {
        self.0.id = Some(id);
        self
    }

    /// Sets the text input's maximum input length.
    pub const fn max_length(mut self, max: u16) -> Self {
        self.0.max_length = Some(max);
        self
    }

    /// Sets the text input's minimum input length.
    pub const fn min_length(mut self, min: u16) -> Self {
        self.0.min_length = Some(min);
        self
    }

    /// Sets the text input's placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.0.placeholder = Some(placeholder.into());
        self
    }

    /// Sets whether the button is required.
    pub const fn required(mut self, required: bool) -> Self {
        self.0.required = Some(required);
        self
    }

    /// Sets the text input's pre-filled value text.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.0.value = Some(value.into());
        self
    }

    /// Builds the completed text input.
    #[must_use]
    pub fn build(self) -> TextInput {
        self.0
    }
}

impl From<TextInputBuilder> for TextInput {
    fn from(value: TextInputBuilder) -> Self {
        value.build()
    }
}

impl From<TextInputBuilder> for Component {
    fn from(value: TextInputBuilder) -> Self {
        Self::TextInput(value.build())
    }
}
