use std::time::{Duration, Instant};

use reqwest::{
    blocking::{Client, ClientBuilder},
    header::HeaderMap,
};

use crate::{Downloader, Error};

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
