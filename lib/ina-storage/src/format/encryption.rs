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
use std::io::Cursor;
use std::sync::{Arc, OnceLock};

use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{AeadCore, KeyInit, KeySizeUser, XChaCha20Poly1305};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use super::{DataDecode, DataEncode, DataFormat};

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
    /// A header-related error.
    #[error(transparent)]
    Header(#[from] HeaderError),
}

/// An error related to header reading and writing.
#[derive(Debug, thiserror::Error)]
pub enum HeaderError {
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The magic byte sequence did not match.
    #[error("invalid magic byte sequence: {0:02X?}")]
    InvalidMagic([u8; 3]),
    /// The header version did not match.
    #[error("invalid version number: expected {0:02X}, found {1:02X}")]
    InvalidVersion(u8, u8),
}

/// A header used for retaining encryption data.
#[derive(Clone, Debug)]
pub(crate) struct Header {
    /// The salt.
    pub salt: Box<[u8]>,
    /// The nonce.
    pub nonce: Box<[u8]>,
}

impl Header {
    /// The header's magic byte sequence.
    pub const MAGIC: [u8; 3] = *b"1N4";
    /// The header's format version.
    pub const VERSION: u8 = 1;

    /// Creates a new [`Header`].
    pub const fn new(salt: Box<[u8]>, nonce: Box<[u8]>) -> Self {
        Self { salt, nonce }
    }

    /// Returns the total length of the header in bytes.
    pub const fn len(&self) -> usize {
        const USIZE: usize = (usize::BITS / u8::BITS) as usize;

        Self::MAGIC.len() + 1 + USIZE + self.salt.len() + USIZE + self.nonce.len()
    }

    /// Reads a header from the given buffer.
    ///
    /// # Errors
    ///
    /// This function will return an error if reading fails.
    pub fn read_from<R: std::io::Read>(f: &mut R) -> Result<Self, HeaderError> {
        // Extract magic byte sequence.
        let mut magic = [0_u8; Self::MAGIC.len()];
        f.read_exact(&mut magic)?;

        if magic != Self::MAGIC {
            return Err(HeaderError::InvalidMagic(magic));
        }

        // Extract format version information.
        let mut version = [0_u8; 1];
        f.read_exact(&mut version)?;

        if version[0] != Self::VERSION {
            return Err(HeaderError::InvalidVersion(Self::VERSION, version[0]));
        }

        // Extract encryption hashing salt.
        let mut salt_len = [0_u8; (usize::BITS / u8::BITS) as usize];
        f.read_exact(&mut salt_len)?;
        let salt_len = usize::from_le_bytes(salt_len);

        let mut salt = vec![0_u8; salt_len];
        f.read_exact(&mut salt)?;

        // Extract encryption encoding nonce.
        let mut nonce_len = [0_u8; (usize::BITS / u8::BITS) as usize];
        f.read_exact(&mut nonce_len)?;
        let nonce_len = usize::from_le_bytes(nonce_len);

        let mut nonce = vec![0_u8; nonce_len];
        f.read_exact(&mut nonce)?;

        Ok(Self::new(salt.into_boxed_slice(), nonce.into_boxed_slice()))
    }

    /// Writes this header into a given buffer.
    ///
    /// # Errors
    ///
    /// This function will return an error if writing fails.
    pub fn write_into<W: std::io::Write>(&self, f: &mut W) -> std::io::Result<()> {
        f.write_all(&Self::MAGIC)?;
        f.write_all(&[Self::VERSION])?;
        f.write_all(&self.salt.len().to_le_bytes())?;
        f.write_all(&self.salt)?;
        f.write_all(&self.nonce.len().to_le_bytes())?;
        f.write_all(&self.nonce)
    }
}

impl Zeroize for Header {
    fn zeroize(&mut self) {
        self.salt.zeroize();
        self.nonce.zeroize();
    }
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
}

impl<F: Debug + DataFormat + 'static> DataFormat for Encrypt<F> {
    fn extension(&self) -> impl AsRef<OsStr> {
        format!("{}.cha", self.inner.extension().as_ref().to_string_lossy())
    }
}

impl<F: Debug + DataFormat + 'static> DataEncode for Encrypt<F> {
    type Error = Error<F>;

    fn encode<T: Serialize>(&self, value: &T) -> Result<Arc<[u8]>, Self::Error> {
        // Fully serialize the value.
        let bytes = self.inner.encode(value).map_err(Error::Encode)?;

        // Hash the configured password.
        let salt = SaltString::generate(OsRng).to_string().into_bytes();
        let key = self::get_encryption_key(&salt)?;

        // Encode the data using the password hash.
        let nonce = XChaCha20Poly1305::generate_nonce(OsRng);
        let bytes = XChaCha20Poly1305::new((**key).into()).encrypt(&nonce, &*bytes).map_err(Error::ChaCha20Poly1305)?;

        // Create the final output buffer.
        let header = Zeroizing::new(Header::new(salt.into_boxed_slice(), (*nonce).into()));
        let mut output = Vec::with_capacity(header.len() + bytes.len());

        header.write_into(&mut output)?;
        output.extend_from_slice(&bytes);

        Ok(output.into())
    }
}

impl<F: Debug + DataFormat + 'static> DataDecode for Encrypt<F> {
    type Error = Error<F>;

    fn decode<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> Result<T, Self::Error> {
        // Extract the encryption data header.
        let mut reader = Cursor::new(bytes);
        let header = Zeroizing::new(Header::read_from(&mut reader)?);

        // Hash the configured password.
        let key = self::get_encryption_key(&header.salt)?;

        // Decode the data using the password hash.
        let bytes = &bytes[header.len() ..];
        let bytes = XChaCha20Poly1305::new((**key).into())
            .decrypt((*header.nonce).into(), bytes)
            .map_err(Error::ChaCha20Poly1305)?;

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

/// Returns a new [`Argon2`].
fn create_argon2<'key>() -> Argon2<'key> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
}

/// Returns the configured password if available.
fn get_password<F: Debug + DataFormat>() -> Result<Zeroizing<String>, Error<F>> {
    PASSWORD_RESOLVER.get().and_then(|f| f()).map(Zeroizing::new).ok_or(Error::MissingPassword)
}

/// Returns an encryption key based on the given salt and the configured password.
///
/// # Errors
///
/// This function will return an error if the password is not set or hashing fails.
fn get_encryption_key<F: Debug + DataFormat>(salt: &[u8]) -> Result<Zeroizing<Box<[u8]>>, Error<F>> {
    let mut key = vec![0_u8; XChaCha20Poly1305::key_size()];
    let password = self::get_password()?;

    self::create_argon2().hash_password_into(password.as_bytes(), salt, &mut key).map_err(Error::Argon2)?;

    Ok(Zeroizing::new(key.into_boxed_slice()))
}
