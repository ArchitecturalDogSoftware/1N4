// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2025 Jaxydog
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

use std::ffi::OsStr;
use std::fmt::Debug;
use std::io::{Cursor, Read};
use std::sync::{Arc, OnceLock};

use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{AeadCore, KeyInit, KeySizeUser, XChaCha20Poly1305};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use super::{DataDecode, DataEncode, DataFormat};

/// The bytes used as magic numbers for the encrypted file headers.
const MAGIC: &[u8] = b"1N4";
/// The number of bytes required to store one usize.
const USIZE_LEN: usize = (usize::BITS / u8::BITS) as usize;

/// The function used to resolve the encryption password at runtime.
static PASSWORD_RESOLVER: OnceLock<fn() -> Option<String>> = OnceLock::new();

/// An encryption format error.
#[derive(Debug, thiserror::Error)]
pub enum Error<F: Debug + DataFormat> {
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An Argon2 error.
    #[error("failed to create hashed password")]
    Argon2(argon2::Error),
    /// A `ChaCha20Poly1305` error.
    #[error("failed to encrypt/decrypt data")]
    ChaCha20Poly1305(chacha20poly1305::Error),
    /// An encoding error.
    #[error(transparent)]
    Encode(<F as DataEncode>::Error),
    /// A decoding error.
    #[error(transparent)]
    Decode(<F as DataDecode>::Error),
    /// A password was not set.
    #[error("a password was not set")]
    MissingPassword,
    /// The given bytes are missing the magic header.
    #[error("missing magic byte header")]
    InvalidMagic,
}

/// Encrypts the wrapped format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Encrypt<F: Debug + DataFormat> {
    /// The inner format.
    inner: F,
}

impl<F: Debug + DataFormat> Encrypt<F> {
    /// Creates a new [`Encrypt<F>`].
    pub const fn new(inner: F) -> Self {
        Self { inner }
    }

    /// Returns a new [`Argon2`].
    fn new_argon2<'key>() -> Argon2<'key> {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
    }

    /// Returns the configured password if available.
    fn password() -> Result<String, Error<F>> {
        PASSWORD_RESOLVER.get().and_then(|f| f()).ok_or(Error::MissingPassword)
    }
}

impl<F: Debug + DataFormat + 'static> DataFormat for Encrypt<F> {
    fn extension(&self) -> impl AsRef<OsStr> {
        format!("{}.cha", self.inner.extension().as_ref().to_string_lossy())
    }
}

impl<F: Debug + DataFormat + 'static> DataEncode for Encrypt<F> {
    type Error = Error<F>;

    fn encode<T: Serialize>(&self, value: &T) -> Result<Arc<[u8]>, Self::Error> {
        let bytes = self.inner.encode(value).map_err(Error::Encode)?;

        // Hash the configured password.
        let mut salt = SaltString::generate(OsRng).to_string().into_bytes();
        let mut key = vec![0_u8; XChaCha20Poly1305::key_size()];

        let mut password = Self::password()?.into_bytes();

        Self::new_argon2().hash_password_into(&password, &salt, &mut key).map_err(Error::Argon2)?;

        password.zeroize();

        // Encode the data using the password hash.
        let mut nonce = XChaCha20Poly1305::generate_nonce(OsRng);
        let cipher = XChaCha20Poly1305::new((*key).into());

        key.zeroize();

        let bytes = cipher.encrypt(&nonce, &*bytes).map_err(Error::ChaCha20Poly1305)?;

        let capacity = MAGIC.len() + (USIZE_LEN * 2) + salt.len() + nonce.len() + bytes.len();

        // Create the final output buffer.
        // Encrypted files are formatted as follows: [magic][salt len][salt][nonce len][nonce][data]
        let mut output = Vec::with_capacity(capacity);

        output.extend_from_slice(MAGIC);

        output.extend_from_slice(&salt.len().to_le_bytes());
        output.extend_from_slice(&salt);
        salt.zeroize();

        output.extend_from_slice(&nonce.len().to_le_bytes());
        output.extend_from_slice(&nonce);
        nonce.zeroize();

        output.extend_from_slice(&bytes);

        Ok(output.into())
    }
}

impl<F: Debug + DataFormat + 'static> DataDecode for Encrypt<F> {
    type Error = Error<F>;

    fn decode<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> Result<T, Self::Error> {
        // Strip the magic byte prefix.
        let Some(bytes) = bytes.strip_prefix(MAGIC) else {
            return Err(Error::InvalidMagic);
        };

        // Split up the encoded data.
        let mut reader = Cursor::new(bytes);
        let mut usize_buffer = [0_u8; USIZE_LEN];

        reader.read_exact(&mut usize_buffer)?;
        let salt_len = usize::from_le_bytes(usize_buffer);
        let mut salt = vec![0_u8; salt_len];
        reader.read_exact(&mut salt)?;

        reader.read_exact(&mut usize_buffer)?;
        let nonce_len = usize::from_le_bytes(usize_buffer);
        let mut nonce = vec![0_u8; nonce_len];
        reader.read_exact(&mut nonce)?;

        // Hash the configured password.
        let mut key = vec![0_u8; XChaCha20Poly1305::key_size()];
        let mut password = Self::password()?.into_bytes();

        Self::new_argon2().hash_password_into(&password, &salt, &mut key).map_err(Error::Argon2)?;

        password.zeroize();
        salt.zeroize();

        // Decode the data using the hashed password.
        let cipher = XChaCha20Poly1305::new((*key).into());

        key.zeroize();

        let bytes = &bytes[((USIZE_LEN * 2) + salt_len + nonce_len) ..];
        let bytes = cipher.decrypt((*nonce).into(), bytes).map_err(Error::ChaCha20Poly1305)?;

        nonce.zeroize();

        self.inner.decode(&bytes).map_err(Error::Decode)
    }
}

/// Sets the password resolver of all [`Encrypt<F>`] formats.
///
/// # Panics
///
/// Panics if the resolver was already set.
#[expect(clippy::expect_used, reason = "we should fail if the resolver is set multiple times")]
pub fn set_password_resolver(f: fn() -> Option<String>) {
    PASSWORD_RESOLVER.set(f).expect("the password resolver has already been set");
}
