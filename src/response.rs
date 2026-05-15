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

    pub(crate) fn new_bytes_response(bytes: Bytes, headers: HeaderMap) -> Self {
        Response::BytesResponse(BytesResponse { bytes, headers })
    }

    pub(crate) fn new_save_to_file_response(size: u64, headers: HeaderMap) -> Self {
        Response::SaveToFileResponse(SaveToFileResponse { headers, size })
    }
}

// ======================================================================
// BytesResponse - PUBLIC
// ======================================================================

/// Response returned by [`RequestBuilder::get`].
pub struct BytesResponse {
    bytes: Bytes,
    headers: HeaderMap,
}

impl BytesResponse {
    /// Returns downloaded file.
    pub fn bytes(&self) -> &Bytes {
        &self.bytes
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
    headers: HeaderMap,
    size: u64,
}

impl SaveToFileResponse {
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
