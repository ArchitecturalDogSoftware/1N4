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

use twilight_model::channel::message::Component;
use twilight_model::channel::message::component::{
    MediaGallery, MediaGalleryItem, TextInput, TextInputStyle, UnfurledMediaItem,
};
use twilight_validate::component::{self as validation, ComponentValidationError};

use crate::utility::traits::extension::UnfurledMediaItemExt;

/// An aliased result type that returns a [`ComponentValidationError`] as its error type.
type Result<T> = std::result::Result<T, ComponentValidationError>;

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
    pub fn item(mut self, item: impl Into<MediaGalleryItem>) -> Result<Self> {
        self.0.items.push(item.into());

        self::validation::media_gallery(&self.0).map(|()| self)
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
    pub fn description(mut self, description: impl Into<String>) -> Result<Self> {
        self.0.description = Some(description.into());

        self::validation::media_gallery_item(&self.0).map(|()| self)
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
    pub fn new(custom_id: impl Into<String>, style: TextInputStyle) -> Result<Self> {
        #[expect(deprecated, reason = "we still need to set the field, even if it's just to `None`")]
        let inner = TextInput {
            id: None,
            custom_id: custom_id.into(),
            label: None,
            max_length: None,
            min_length: None,
            placeholder: None,
            required: None,
            style,
            value: None,
        };

        self::validation::text_input(&inner, true).map(|()| Self(inner))
    }

    /// Sets the text input's numeric identifier.
    pub const fn id(mut self, id: i32) -> Self {
        self.0.id = Some(id);
        self
    }

    /// Sets the text input's maximum input length.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value is outside of the valid range.
    pub fn max_length(mut self, max: u16) -> Result<Self> {
        self.0.max_length = Some(max);

        self::validation::text_input(&self.0, true).map(|()| self)
    }

    /// Sets the text input's minimum input length.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value is outside of the valid range.
    pub fn min_length(mut self, min: u16) -> Result<Self> {
        self.0.min_length = Some(min);

        self::validation::text_input(&self.0, true).map(|()| self)
    }

    /// Sets the text input's placeholder text.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value's length exceeds the limit.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Result<Self> {
        self.0.placeholder = Some(placeholder.into());

        self::validation::text_input(&self.0, true).map(|()| self)
    }

    /// Sets whether the button is required.
    pub const fn required(mut self, required: bool) -> Self {
        self.0.required = Some(required);

        self
    }

    /// Sets the text input's pre-filled value text.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value's length exceeds the limit.
    pub fn value(mut self, value: impl Into<String>) -> Result<Self> {
        self.0.value = Some(value.into());

        self::validation::text_input(&self.0, true).map(|()| self)
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
