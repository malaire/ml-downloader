#![doc = include_str!(concat!(env!("OUT_DIR"), "/README-rustdocified.md"))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::{
    error::Error as StdError,
    fmt, thread,
    time::{Duration, Instant},
};

use bytes::Bytes;
use digest::DynDigest;
use reqwest::{
    blocking::{Client, ClientBuilder},
    Error as ReqwestError, IntoUrl, StatusCode,
};

// ======================================================================
// Error - PUBLIC

/// Represents all possible errors that can occur in this library.
#[derive(Debug)]
pub enum Error {
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

    /// Download failed.
    DownloadFailed(
        /// Errors, one error for each (re)try.
        Vec<Error>,
    ),
}

// ======================================================================
// Error - IMPL DISPLAY

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Reqwest(inner) => inner.fmt(f),
            Error::StatusNotOk(status) => status.fmt(f),
            Error::HashMismatch { got, expected } => {
                write!(f, "hash mismatch\nGot     :{}\nExpected:{}", got, expected)
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

impl StdError for Error {}

// ======================================================================
// Error - IMPL FROM

impl From<ReqwestError> for Error {
    fn from(error: ReqwestError) -> Self {
        Self::Reqwest(error)
    }
}

// ======================================================================
// Downloader - PUBLIC

/// Simple blocking downloader.
///
/// See [crate index](crate#examples) for examples.
pub struct Downloader {
    client: Client,
    min_interval: Duration,
    max_interval: Duration,
    retry_delays: Vec<(Duration, Duration)>,
    prev_download_start: Option<Instant>,
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

    /// Begins building a request to download file from given `url`.
    ///
    /// See [simple usage] and [`RequestBuilder::hash`] for examples.
    ///
    /// [simple usage]: crate#simple-usage
    pub fn get<U: IntoUrl>(&mut self, url: U) -> RequestBuilder {
        RequestBuilder::new(self, self.client.get(url))
    }

    /// Creates new [`Downloader`] with default configuration.
    pub fn new() -> Result<Self, Error> {
        DownloaderBuilder::new().build()
    }

    /// Sleeps until ready for next download.
    ///
    /// After this the next [`RequestBuilder::send`] will start
    /// download immediately without sleep.
    ///
    /// See [`DownloaderBuilder::interval`].
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
    /// let bytes1 = downloader.get("https://example.com/first").send()?;
    /// downloader.sleep_until_ready();
    /// println!("Second download");
    /// let bytes2 = downloader.get("https://example.com/second").send()?;
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    pub fn sleep_until_ready(&mut self) {
        if let Some(prev_download_start) = self.prev_download_start {
            let interval = random_duration(self.min_interval, self.max_interval);
            let elapsed = Instant::now() - prev_download_start;
            if elapsed < interval {
                std::thread::sleep(interval - elapsed);
            }
            self.prev_download_start = None;
        }
    }
}

// ======================================================================
// DownloaderBuilder - PUBLIC

/// A builder to create [`Downloader`] with custom configuration.
///
/// See [custom configuration] for an example.
///
/// [custom configuration]: crate#custom-configuration
pub struct DownloaderBuilder {
    client_builder: ClientBuilder,
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
            min_interval: self.min_interval,
            max_interval: self.max_interval,
            retry_delays: self.retry_delays,
            prev_download_start: None,
        })
    }

    /// Sets interval between successful downloads in seconds, default is 0.
    ///
    /// A random interval between given `min` and `max` is generated
    /// for each download. If elapsed time since previous download started
    /// is less than this interval then [`RequestBuilder::send`] will sleep
    /// for the remaining duration before starting download.
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

// ======================================================================
// RequestBuilder - PUBLIC

/// A builder to configure download request.
///
/// See [custom configuration] for an example.
///
/// [custom configuration]: crate#custom-configuration
pub struct RequestBuilder<'a> {
    downloader: &'a mut Downloader,
    inner: reqwest::blocking::RequestBuilder,
    hash: Option<(String, Box<dyn DynDigest>)>,
}

impl<'a> RequestBuilder<'a> {
    /// Sets expected file hash and digest used to calculate it.
    ///
    /// Hash is given in hexadecimal, uppercase or lowercase.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ml_downloader::Downloader;
    /// use sha2::{Digest, Sha256};
    ///
    /// let mut downloader = Downloader::new()?;
    /// let bytes = downloader
    ///     .get("https://example.com/")
    ///     .hash("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", Sha256::new())
    ///     .send()?;
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    pub fn hash<D: DynDigest + 'static>(self, expected: &str, digest: D) -> Self {
        RequestBuilder {
            hash: Some((expected.to_lowercase(), Box::new(digest))),
            ..self
        }
    }

    /// Creates download request and sends it to target URL, with retries.
    ///
    /// - Sleeps before starting download if needed.
    ///     - See [`DownloaderBuilder::interval`] and [`Downloader::sleep_until_ready`].
    /// - Number of retries and the delays inbetween them is configured with
    ///   [`DownloaderBuilder::retry_delays`].
    ///
    /// See [simple usage] and [`RequestBuilder::hash`] for examples.
    ///
    /// [simple usage]: crate#simple-usage
    pub fn send(mut self) -> Result<Bytes, Error> {
        let mut errors = Vec::with_capacity(self.downloader.retry_delays.len());

        self.downloader.sleep_until_ready();

        let mut retry_count = 0;
        loop {
            self.downloader.prev_download_start = Some(Instant::now());

            match self.send_once() {
                Ok(bytes) => return Ok(bytes),
                Err(error) => errors.push(error),
            }

            if retry_count == self.downloader.retry_delays.len() {
                return Err(Error::DownloadFailed(errors));
            }

            let (min, max) = self.downloader.retry_delays[retry_count];
            thread::sleep(random_duration(min, max));
            retry_count += 1;
        }
    }
}

// ======================================================================
// RequestBuilder - PRIVATE

impl<'a> RequestBuilder<'a> {
    fn new(downloader: &'a mut Downloader, inner: reqwest::blocking::RequestBuilder) -> Self {
        Self {
            downloader,
            inner,
            hash: None,
        }
    }

    fn send_once(&mut self) -> Result<Bytes, Error> {
        let response = self.inner.try_clone().unwrap().send()?;
        let status = response.status();

        if status != StatusCode::OK {
            Err(Error::StatusNotOk(status))
        } else {
            let bytes = response.bytes()?;
            if let Some((expected, digest)) = &mut self.hash {
                digest.reset();
                digest.update(&bytes);
                let mut got = vec![0; digest.output_size()];
                digest.finalize_into_reset(got.as_mut()).unwrap();
                let got = hex::encode(got);

                if &got != expected {
                    return Err(Error::HashMismatch {
                        got,
                        expected: expected.clone(),
                    });
                }
            }
            Ok(bytes)
        }
    }
}

// ======================================================================
// FUNCTIONS - PRIVATE

fn random_duration(min: Duration, max: Duration) -> Duration {
    Duration::from_micros(fastrand::u64(
        min.as_micros() as u64..=max.as_micros() as u64,
    ))
}
