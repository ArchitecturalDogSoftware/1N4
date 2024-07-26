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

/// Contains the HTTP API and its cache.
#[derive(Clone, Debug)]
pub struct Api {
    /// The HTTP client.
    pub client: Arc<Client>,
    /// The cache.
    pub cache: Arc<DefaultInMemoryCache>,
}

impl Api {
    /// Creates a new [`Api`] with an empty cache.
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self { client: Arc::new(client), cache: Arc::new(DefaultInMemoryCache::new()) }
    }

    /// Returns a reference to this [`Api`].
    #[must_use]
    pub const fn as_ref(&self) -> ApiRef {
        ApiRef { client: &self.client, cache: &self.cache }
    }

    /// Returns a reference to this [`Api`].
    #[must_use]
    pub fn as_mut(&mut self) -> ApiMut {
        ApiMut { client: &mut self.client, cache: &mut self.cache }
    }
}

/// Contains a reference to the HTTP API and its cache.
#[derive(Clone, Copy, Debug)]
pub struct ApiRef<'api> {
    /// A reference to the HTTP client.
    pub client: &'api Arc<Client>,
    /// A reference to the cache.
    pub cache: &'api Arc<DefaultInMemoryCache>,
}

impl<'api> ApiRef<'api> {
    /// Returns a cloned version of this [`ApiRef`].
    #[must_use]
    pub fn as_owned(&self) -> Api {
        Api { client: Arc::clone(self.client), cache: Arc::clone(self.cache) }
    }
}

/// Contains a mutable reference to the HTTP API and its cache.
#[derive(Debug)]
pub struct ApiMut<'api> {
    /// A mutable reference to the HTTP client.
    pub client: &'api mut Arc<Client>,
    /// A mutable reference to the cache.
    pub cache: &'api mut Arc<DefaultInMemoryCache>,
}

impl<'api> ApiMut<'api> {
    /// Returns a demoted reference to this [`ApiMut`].
    #[must_use]
    pub const fn as_ref(&self) -> ApiRef {
        ApiRef { client: self.client, cache: self.cache }
    }

    /// Returns a cloned version of this [`ApiMut`].
    #[must_use]
    pub fn as_owned(&self) -> Api {
        Api { client: Arc::clone(self.client), cache: Arc::clone(self.cache) }
    }
}
