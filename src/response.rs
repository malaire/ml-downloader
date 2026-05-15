use std::time::SystemTime;

use bytes::Bytes;
use reqwest::header::HeaderMap;

use crate::{util, Error};

// ======================================================================
// Response - CRATE
// ======================================================================

pub(crate) enum Response {
    BytesResponse(BytesResponse),
    SaveToFileResponse(SaveToFileResponse),
}

impl Response {
    // This is only called when `self` is known to be `BytesResponse`.
    pub(crate) fn into_bytes_response_or_panic(self) -> BytesResponse {
        match self {
            Response::BytesResponse(response) => response,
            Response::SaveToFileResponse(_) => panic!("internal error"),
        }
    }

    // This is only called when `self` is known to be `SaveToFileResponse`.
    pub(crate) fn into_save_to_file_response_or_panic(self) -> SaveToFileResponse {
        match self {
            Response::BytesResponse(_) => panic!("internal error"),
            Response::SaveToFileResponse(response) => response,
        }
    }

    pub(crate) fn new_bytes_response(
        bytes: Bytes,
        hash: Option<Vec<u8>>,
        headers: HeaderMap,
    ) -> Self {
        Response::BytesResponse(BytesResponse {
            bytes,
            hash,
            headers,
        })
    }

    pub(crate) fn new_save_to_file_response(
        size: u64,
        hash: Option<Vec<u8>>,
        headers: HeaderMap,
    ) -> Self {
        Response::SaveToFileResponse(SaveToFileResponse {
            headers,
            hash,
            size,
        })
    }
}

// ======================================================================
// BytesResponse - PUBLIC
// ======================================================================

/// Response returned by [`RequestBuilder::get`].
pub struct BytesResponse {
    bytes: Bytes,
    hash: Option<Vec<u8>>,
    headers: HeaderMap,
}

impl BytesResponse {
    /// Returns downloaded file.
    pub fn bytes(&self) -> &Bytes {
        &self.bytes
    }

    /// Returns hash of the downloaded file in lowercase hexadecimal.
    ///
    /// Returns `Some` iff [`RequestBuilder::calculate_hash`] or
    /// [`RequestBuilder::verify_hash`] was used.
    ///
    /// See [`RequestBuilder::calculate_hash`] for example.
    pub fn hash(&self) -> Option<String> {
        if let Some(hash) = &self.hash {
            Some(hex::encode(hash))
        } else {
            None
        }
    }

    /// Returns response headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns parsed Last-Modified header if present or `None` otherwise.
    ///
    /// # Errors
    ///
    /// Fails if Last-Modified header is invalid and can't be parsed.
    pub fn modified(&self) -> Result<Option<SystemTime>, Error> {
        Ok(util::parse_last_modified_header(&self.headers)?)
    }
}

// ======================================================================
// SaveToFileResponse - PUBLIC
// ======================================================================

/// Response returned by [`RequestBuilder::save_to_file`].
pub struct SaveToFileResponse {
    hash: Option<Vec<u8>>,
    headers: HeaderMap,
    size: u64,
}

impl SaveToFileResponse {
    /// Returns hash of the downloaded file in lowercase hexadecimal.
    ///
    /// Returns `Some` iff [`RequestBuilder::calculate_hash`] or
    /// [`RequestBuilder::verify_hash`] was used.
    ///
    /// See [`RequestBuilder::calculate_hash`] for example.
    pub fn hash(&self) -> Option<String> {
        if let Some(hash) = &self.hash {
            Some(hex::encode(hash))
        } else {
            None
        }
    }

    /// Returns response headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns parsed Last-Modified header if present or `None` otherwise.
    ///
    /// # Errors
    ///
    /// Fails if Last-Modified header is invalid and can't be parsed.
    pub fn modified(&self) -> Result<Option<SystemTime>, Error> {
        Ok(util::parse_last_modified_header(&self.headers)?)
    }

    /// Returns size of the downloaded file in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }
}
