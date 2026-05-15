use std::time::{Duration, Instant};

use reqwest::{
    blocking::{Client, Request, Response},
    IntoUrl,
};

use crate::{util, DownloaderBuilder, Error, RequestBuilder};

// ======================================================================
// Downloader - PUBLIC
// ======================================================================

/// Simple blocking downloader.
///
/// See [crate index](crate#examples) for examples.
pub struct Downloader {
    client: Client,
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

    /// Creates new [`Downloader`] with default configuration.
    pub fn new() -> Result<Self, Error> {
        Self::from_builder(DownloaderBuilder::new())
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
    /// let response1 = downloader.url("https://example.com/first").get()?;
    /// downloader.sleep_until_ready();
    /// println!("Second download");
    /// let response2 = downloader.url("https://example.com/second").get()?;
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
// Downloader - CRATE
// ======================================================================

impl Downloader {
    pub(crate) fn execute(&self, request: Request) -> Result<Response, Error> {
        Ok(self.client.execute(request)?)
    }

    pub(crate) fn from_builder(builder: DownloaderBuilder) -> Result<Self, Error> {
        Ok(Downloader {
            client: builder.client_builder.build()?,
            min_interval: builder.min_interval,
            max_interval: builder.max_interval,
            min_delay: builder.min_delay,
            max_delay: builder.max_delay,
            retry_delays: builder.retry_delays,
            sleep_until: Instant::now(),
        })
    }

    pub(crate) fn retry_delays(&self) -> &[(Duration, Duration)] {
        &self.retry_delays
    }

    pub(crate) fn update_sleep_until(&mut self, download_start: Instant, download_end: Instant) {
        let delay = util::random_duration(self.min_delay, self.max_delay);
        let interval = util::random_duration(self.min_interval, self.max_interval);
        self.sleep_until = (download_start + interval).max(download_end + delay);
    }
}
