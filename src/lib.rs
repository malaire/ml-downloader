#![doc = include_str!(concat!(env!("OUT_DIR"), "/README-rustdocified.md"))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::{
    error::Error as StdError,
    fmt,
    io::Error as IoError,
    time::{Duration, Instant, SystemTime},
};

use reqwest::{
    blocking::{Client, ClientBuilder},
    header::HeaderMap,
    Error as ReqwestError, IntoUrl, StatusCode,
};

pub use crate::request_builder::RequestBuilder;

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

// ======================================================================
// Downloader - PUBLIC
// ======================================================================

/// Simple blocking downloader.
///
/// See [crate index](crate#examples) for examples.
pub struct Downloader {
    client: Client,
    headers: HeaderMap,
    min_delay: Duration,
    max_delay: Duration,
    min_interval: Duration,
    max_interval: Duration,
    retry_delays: Vec<(Duration, Duration)>,
    sleep_until: Instant,
}

impl Downloader {
    /// Creates [`DownloaderBuilder`] to configure [`Downloader`].
    ///
    /// This is same as [`DownloaderBuilder::new`].
    ///
    /// See [custom configuration] for an example.
    ///
    /// [custom configuration]: crate#custom-configuration
    pub fn builder() -> DownloaderBuilder {
        DownloaderBuilder::new()
    }

    /// Returns response headers of the latest download.
    ///
    /// Returned [`HeaderMap`] is empty if the latest download failed
    /// before getting headers or if no downloads have been done yet.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ml_downloader::Downloader;
    ///
    /// let mut downloader = Downloader::new()?;
    /// let bytes = downloader.url("https://example.com/").get()?;
    /// let headers = downloader.headers();
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    ///
    /// [`HeaderMap`]: reqwest::header::HeaderMap
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns parsed Last-Modified header of the latest download.
    ///
    /// Returns `None` if
    /// - the latest download didn't have Last-Modified header
    /// - the latest download failed before getting headers
    /// - no downloads have been done yet
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ml_downloader::Downloader;
    ///
    /// let mut downloader = Downloader::new()?;
    /// let bytes = downloader.url("https://example.com/").get()?;
    /// let mtime = downloader.modified()?.unwrap();
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if Last-Modified header is invalid and can't be parsed.
    pub fn modified(&self) -> Result<Option<SystemTime>, Error> {
        Ok(util::parse_last_modified_header(&self.headers)?)
    }

    /// Creates new [`Downloader`] with default configuration.
    pub fn new() -> Result<Self, Error> {
        DownloaderBuilder::new().build()
    }

    /// Sleeps until ready for next download.
    ///
    /// After this the next [`RequestBuilder::get`] will start
    /// download immediately without sleep.
    ///
    /// See [`DownloaderBuilder::delay`] and [`DownloaderBuilder::interval`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ml_downloader::Downloader;
    ///
    /// let mut downloader = Downloader::builder()
    ///     .interval(1.0, 1.0)
    ///     .build()?;
    ///
    /// println!("First download");
    /// let bytes1 = downloader.url("https://example.com/first").get()?;
    /// downloader.sleep_until_ready();
    /// println!("Second download");
    /// let bytes2 = downloader.url("https://example.com/second").get()?;
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    pub fn sleep_until_ready(&mut self) {
        let now = Instant::now();
        if self.sleep_until > now {
            std::thread::sleep(self.sleep_until - now);
        }
    }

    /// Begins building a request to download file from given `url`.
    ///
    /// See [simple usage] and [`RequestBuilder::hash`] for examples.
    ///
    /// # Errors
    ///
    /// If given `url` is invalid then [`RequestBuilder::get`] will fail.
    ///
    /// [simple usage]: crate#simple-usage
    pub fn url<U: IntoUrl>(&mut self, url: U) -> RequestBuilder<'_> {
        RequestBuilder::new(self, self.client.get(url))
    }
}

// ======================================================================
// DownloaderBuilder - PUBLIC
// ======================================================================

/// A builder to create [`Downloader`] with custom configuration.
///
/// See [custom configuration] for an example.
///
/// [custom configuration]: crate#custom-configuration
pub struct DownloaderBuilder {
    client_builder: ClientBuilder,
    min_delay: Duration,
    max_delay: Duration,
    min_interval: Duration,
    max_interval: Duration,
    retry_delays: Vec<(Duration, Duration)>,
}

impl Default for DownloaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloaderBuilder {
    /// Creates [`Downloader`] using configuration of this [`DownloaderBuilder`].
    ///
    /// See [custom configuration] for an example.
    ///
    /// [custom configuration]: crate#custom-configuration
    pub fn build(self) -> Result<Downloader, Error> {
        Ok(Downloader {
            client: self.client_builder.build()?,
            headers: HeaderMap::new(),
            min_interval: self.min_interval,
            max_interval: self.max_interval,
            min_delay: self.min_delay,
            max_delay: self.max_delay,
            retry_delays: self.retry_delays,
            sleep_until: Instant::now(),
        })
    }

    /// Sets delay between successful downloads in seconds, default is 0.
    ///
    /// A random delay between given `min` and `max` is generated
    /// for each download. If elapsed time since previous download ended
    /// is less than this delay then [`RequestBuilder::get`] will sleep
    /// for the remaining duration before starting download.
    ///
    /// See also [`DownloaderBuilder::interval`].
    ///
    /// # Panics
    ///
    /// If `min > max`.
    ///
    /// # Examples
    ///
    /// Configure `1.0 - 1.1` seconds delay between successful downloads.
    ///
    /// ```rust
    /// use ml_downloader::Downloader;
    ///
    /// let mut downloader = Downloader::builder()
    ///     .delay(1.0, 1.1)
    ///     .build()?;
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    pub fn delay(self, min: f32, max: f32) -> Self {
        assert!(min <= max);
        DownloaderBuilder {
            min_delay: Duration::from_secs_f32(min),
            max_delay: Duration::from_secs_f32(max),
            ..self
        }
    }

    /// Sets interval between successful downloads in seconds, default is 0.
    ///
    /// A random interval between given `min` and `max` is generated
    /// for each download. If elapsed time since previous download started
    /// is less than this interval then [`RequestBuilder::get`] will sleep
    /// for the remaining duration before starting download.
    ///
    /// See also [`DownloaderBuilder::delay`].
    ///
    /// # Panics
    ///
    /// If `min > max`.
    ///
    /// # Examples
    ///
    /// Configure `1.0 - 1.1` seconds interval between successful downloads.
    ///
    /// ```rust
    /// use ml_downloader::Downloader;
    ///
    /// let mut downloader = Downloader::builder()
    ///     .interval(1.0, 1.1)
    ///     .build()?;
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    pub fn interval(self, min: f32, max: f32) -> Self {
        assert!(min <= max);
        DownloaderBuilder {
            min_interval: Duration::from_secs_f32(min),
            max_interval: Duration::from_secs_f32(max),
            ..self
        }
    }

    /// Creates [`DownloaderBuilder`] to configure [`Downloader`].
    ///
    /// This is same as [`Downloader::builder`].
    pub fn new() -> Self {
        Self {
            client_builder: Client::builder(),
            min_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            min_interval: Duration::ZERO,
            max_interval: Duration::ZERO,
            retry_delays: Vec::new(),
        }
    }

    /// Configures underlying [`ClientBuilder`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ml_downloader::Downloader;
    ///
    /// let mut downloader = Downloader::builder()
    ///     .reqwest(|cb| cb.user_agent("foobar/1.0"))
    ///     .build()?;
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    ///
    /// [`ClientBuilder`]: reqwest::blocking::ClientBuilder
    pub fn reqwest<F>(self, f: F) -> Self
    where
        F: FnOnce(ClientBuilder) -> ClientBuilder,
    {
        DownloaderBuilder {
            client_builder: f(self.client_builder),
            ..self
        }
    }

    /// Sets retry delays in seconds, default is none.
    ///
    /// Each item is a pair of `min` and `max` delays
    /// and the number of items defines the number of retries.
    ///
    /// A random delay between given `min` and `max` is generated for each retry.
    ///
    /// # Panics
    ///
    /// If any item has `min > max`.
    ///
    /// # Examples
    ///
    /// Configure two retries after failed download with
    /// `2.0 - 2.2` seconds delay after initial failure and
    /// `5.0 - 5.5` seconds delay after 2nd failure.
    ///
    /// ```rust
    /// use ml_downloader::Downloader;
    ///
    /// let mut downloader = Downloader::builder()
    ///     .retry_delays(&[(2.0, 2.2), (5.0, 5.5)])
    ///     .build()?;
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    pub fn retry_delays(self, retry_delays: &[(f32, f32)]) -> Self {
        let mut vec = Vec::with_capacity(retry_delays.len());
        for (min, max) in retry_delays {
            assert!(min <= max);
            vec.push((Duration::from_secs_f32(*min), Duration::from_secs_f32(*max)));
        }

        DownloaderBuilder {
            retry_delays: vec,
            ..self
        }
    }
}
