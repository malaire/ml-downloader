#![doc = include_str!(concat!(env!("OUT_DIR"), "/README-rustdocified.md"))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::{error::Error as StdError, fmt, io::Error as IoError};

use reqwest::{Error as ReqwestError, StatusCode};

pub use crate::{
    downloader::Downloader, downloader_builder::DownloaderBuilder, request_builder::RequestBuilder,
};

mod downloader;
mod downloader_builder;
mod request_builder;
mod util;

// ======================================================================
// Error - PUBLIC
// ======================================================================

/// Represents all possible errors that can occur in this library.
#[derive(Debug)]
pub enum Error {
    /// Got [`std::io::Error`].
    IoError(IoError),

    /// Got error from [reqwest](https://crates.io/crates/reqwest).
    Reqwest(
        /// The error.
        ReqwestError,
    ),

    /// HTTP response status is not `OK` (200).
    StatusNotOk(
        /// HTTP response status.
        StatusCode,
    ),

    /// Hash of downloaded file doesn't match.
    HashMismatch {
        /// Hash of downloaded file, lowercase hexadecimal.
        got: String,
        /// Hash given to [`RequestBuilder::hash`], lowercase hexadecimal.
        expected: String,
    },

    /// Last-Modified header is invalid and couldn't be parsed.
    InvalidLastModifiedHeader,

    /// Download failed.
    DownloadFailed(
        /// Errors, one error for each (re)try.
        Vec<Error>,
    ),
}

// ======================================================================
// Error - IMPL DISPLAY
// ======================================================================

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IoError(inner) => inner.fmt(f),
            Error::Reqwest(inner) => inner.fmt(f),
            Error::StatusNotOk(status) => status.fmt(f),
            Error::HashMismatch { got, expected } => {
                write!(f, "hash mismatch\nGot     :{}\nExpected:{}", got, expected)
            }
            Error::InvalidLastModifiedHeader => {
                write!(f, "invalid Last-Modified header")
            }
            Error::DownloadFailed(errors) => {
                write!(f, "download failed:")?;
                for (index, error) in errors.iter().enumerate() {
                    write!(f, "\n[{}]: {}", index, error)?;
                }
                Ok(())
            }
        }
    }
}

// ======================================================================
// Error - IMPL ERROR
// ======================================================================

impl StdError for Error {}

// ======================================================================
// Error - IMPL FROM
// ======================================================================

impl From<IoError> for Error {
    fn from(error: IoError) -> Self {
        Self::IoError(error)
    }
}

impl From<ReqwestError> for Error {
    fn from(error: ReqwestError) -> Self {
        Self::Reqwest(error)
    }
}
