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

use std::sync::Arc;

use twilight_cache_inmemory::DefaultInMemoryCache;
use twilight_http::Client;

use super::settings::Settings;

/// Contains the HTTP API and its cache.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Api {
    /// The bot client's settings.
    pub settings: Arc<Settings>,
    /// The HTTP client.
    pub client: Arc<Client>,
    /// The cache.
    pub cache: Arc<DefaultInMemoryCache>,
}

impl Api {
    /// Creates a new [`Api`] with an empty cache.
    #[must_use]
    pub fn new(settings: Settings, client: Client) -> Self {
        Self { settings: Arc::new(settings), client: Arc::new(client), cache: Arc::new(DefaultInMemoryCache::new()) }
    }

    /// Returns a reference to this [`Api`].
    #[must_use]
    pub const fn as_ref(&self) -> ApiRef {
        ApiRef { settings: &self.settings, client: &self.client, cache: &self.cache }
    }
}

/// Contains a reference to the HTTP API and its cache.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct ApiRef<'api> {
    /// The bot client's settings.
    pub settings: &'api Arc<Settings>,
    /// A reference to the HTTP client.
    pub client: &'api Arc<Client>,
    /// A reference to the cache.
    pub cache: &'api Arc<DefaultInMemoryCache>,
}

impl ApiRef<'_> {
    /// Returns a cloned version of this [`ApiRef`].
    #[must_use]
    pub fn into_owned(&self) -> Api {
        Api { settings: Arc::clone(self.settings), client: Arc::clone(self.client), cache: Arc::clone(self.cache) }
    }
}
