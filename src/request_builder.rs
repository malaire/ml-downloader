use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    thread,
    time::Instant,
};

use bytes::Bytes;
use digest::DynDigest;
use reqwest::StatusCode;

// "unused" imports are used with `cargo doc`
#[allow(unused_imports)]
use crate::{
    response::Response, util, Downloader, DownloaderBuilder, Error, GetResponse, SaveToFileResponse,
};

// ======================================================================
// CONST - PRIVATE
// ======================================================================

const BUFFER_SIZE_BYTES: usize = 64 * 1024;
const PARTIAL_DOWNLOAD_FILE_EXTENSION: &str = "part";

// ======================================================================
// RequestBuilder - PUBLIC
// ======================================================================

/// A builder to configure download request.
///
/// See [custom configuration] for an example.
///
/// [custom configuration]: crate#custom-configuration
pub struct RequestBuilder<'a> {
    downloader: &'a mut Downloader,
    inner: reqwest::blocking::RequestBuilder,

    digest: Option<Box<dyn DynDigest>>,
    expected_hash: Option<String>,
}

impl<'a> RequestBuilder<'a> {
    /// Enables hash calculation during download.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ml_downloader::Downloader;
    /// use sha2::{Digest, Sha256};
    ///
    /// let mut downloader = Downloader::new()?;
    /// let response = downloader
    ///     .url("https://example.com/")
    ///     .calculate_hash(Sha256::new())
    ///     .get()?;
    /// let hash = response.hash();
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    pub fn calculate_hash<D: DynDigest + 'static>(self, digest: D) -> Self {
        RequestBuilder {
            digest: Some(Box::new(digest)),
            ..self
        }
    }

    /// Downloads the file into RAM and returns it within [`GetResponse`].
    ///
    /// - Sleeps before starting download if needed.
    ///   - See [`DownloaderBuilder::delay`], [`DownloaderBuilder::interval`]
    ///     and [`Downloader::sleep_until_ready`].
    /// - Number of retries and the delays inbetween them is configured with
    ///   [`DownloaderBuilder::retry_delays`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ml_downloader::Downloader;
    ///
    /// let mut downloader = Downloader::new()?;
    /// let response = downloader.url("https://example.com/").get()?;
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    pub fn get(self) -> Result<GetResponse, Error> {
        Ok(self.download(None::<&Path>)?.into_bytes_response_or_panic())
    }

    /// Downloads the file into RAM and returns it as [`Bytes`].
    ///
    /// - Sleeps before starting download if needed.
    ///   - See [`DownloaderBuilder::delay`], [`DownloaderBuilder::interval`]
    ///     and [`Downloader::sleep_until_ready`].
    /// - Number of retries and the delays inbetween them is configured with
    ///   [`DownloaderBuilder::retry_delays`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ml_downloader::Downloader;
    ///
    /// let mut downloader = Downloader::new()?;
    /// let bytes = downloader.url("https://example.com/").get_bytes()?;
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    pub fn get_bytes(self) -> Result<Bytes, Error> {
        Ok(self.get()?.into_bytes())
    }

    /// Downloads the file and saves it to given path.
    ///
    /// - During download the partially downloaded file is saved with temporary name `{path}.part`.
    ///   - Temporary file is removed if it exists already and also if download fails.
    /// - Once download succeeds file is renamed to `{path}`.
    /// - File modification time is set to Last Modified header, if present.
    /// - Sleeps before starting download if needed.
    ///     - See [`DownloaderBuilder::delay`], [`DownloaderBuilder::interval`]
    ///       and [`Downloader::sleep_until_ready`].
    /// - Number of retries and the delays inbetween them is configured with
    ///   [`DownloaderBuilder::retry_delays`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ml_downloader::Downloader;
    ///
    /// let mut downloader = Downloader::new()?;
    /// let response = downloader
    ///     .url("https://example.com/")
    ///     .save_to_file("example.html")?;
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    ///
    pub fn save_to_file(self, path: impl AsRef<Path>) -> Result<SaveToFileResponse, Error> {
        let path = path.as_ref();

        let download_path = path.with_added_extension(PARTIAL_DOWNLOAD_FILE_EXTENSION);

        if download_path.exists() {
            std::fs::remove_file(&download_path)?;
        }

        let response = self
            .download(Some(&download_path))?
            .into_save_to_file_response_or_panic();

        std::fs::rename(download_path, path)?;

        Ok(response)
    }

    /// Enables hash verification during download.
    ///
    /// Hash is calculated during download and if it differs from given hash
    /// then [`RequestBuilder::get`] or [`RequestBuilder::save_to_file`]
    /// will fail with [`Error::HashMismatch`].
    ///
    /// Hash is given in hexadecimal, case is ignored.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ml_downloader::Downloader;
    /// use sha2::{Digest, Sha256};
    ///
    /// let mut downloader = Downloader::new()?;
    /// let response = downloader
    ///     .url("https://example.com/")
    ///     .verify_hash(
    ///         "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ///         Sha256::new(),
    ///     )
    ///     .get()?;
    ///
    /// # Ok::<(), ml_downloader::Error>(())
    /// ```
    pub fn verify_hash<D: DynDigest + 'static>(self, expected: &str, digest: D) -> Self {
        RequestBuilder {
            digest: Some(Box::new(digest)),
            expected_hash: Some(expected.to_lowercase()),
            ..self
        }
    }
}

// ======================================================================
// RequestBuilder - CRATE
// ======================================================================

impl<'a> RequestBuilder<'a> {
    pub(crate) fn new(
        downloader: &'a mut Downloader,
        inner: reqwest::blocking::RequestBuilder,
    ) -> Self {
        Self {
            downloader,
            inner,
            digest: None,
            expected_hash: None,
        }
    }
}

// ======================================================================
// RequestBuilder - PRIVATE
// ======================================================================

impl<'a> RequestBuilder<'a> {
    fn download(mut self, path: Option<impl AsRef<Path>>) -> Result<Response, Error> {
        let request = self.inner.build()?;
        let mut errors = Vec::with_capacity(self.downloader.retry_delays().len());

        self.downloader.sleep_until_ready();

        let mut retry_count = 0;
        loop {
            let start = Instant::now();

            // `try_clone` can return `None` only if body isn't clonable,
            // but this code never sets body, so this `unwrap` can't fail.
            match RequestBuilder::download_once(
                self.downloader,
                &mut self.digest,
                &self.expected_hash,
                request.try_clone().unwrap(),
                &path,
            ) {
                Ok(result) => {
                    self.downloader.update_sleep_until(start, Instant::now());
                    return Ok(result);
                }
                Err(error) => {
                    errors.push(error);

                    if let Some(ref path) = path {
                        if path.as_ref().exists() {
                            if let Err(error) = std::fs::remove_file(path) {
                                errors.push(error.into());
                            }
                        }
                    }
                }
            }

            let retry_delays = self.downloader.retry_delays();

            if retry_count == retry_delays.len() {
                if errors.len() == 1 {
                    return Err(errors.pop().unwrap());
                } else {
                    return Err(Error::DownloadFailed(errors));
                }
            }

            let (min, max) = retry_delays[retry_count];
            thread::sleep(util::random_duration(min, max));
            retry_count += 1;
        }
    }

    fn download_once(
        downloader: &mut Downloader,
        digest: &mut Option<Box<dyn DynDigest>>,
        expected_hash: &Option<String>,
        request: reqwest::blocking::Request,
        path: &Option<impl AsRef<Path>>,
    ) -> Result<Response, Error> {
        let mut response = downloader.execute(request)?;
        let status = response.status();
        let headers = response.headers().clone();

        if status != StatusCode::OK {
            Err(Error::StatusNotOk(status))
        } else if let Some(path) = path {
            if let Some(digest) = digest {
                digest.reset();
            }

            let mut buffer = [0u8; BUFFER_SIZE_BYTES];
            let mut file = File::create_new(path)?;
            let mut size = 0;

            loop {
                let bytes_read = response.read(&mut buffer)?;

                if bytes_read == 0 {
                    break;
                }

                if let Some(digest) = digest {
                    digest.update(&buffer[..bytes_read]);
                }

                file.write_all(&buffer[..bytes_read])?;
                size += bytes_read;
            }

            let hash = if let Some(digest) = digest {
                let mut hash = vec![0; digest.output_size()];
                digest.finalize_into_reset(hash.as_mut()).unwrap();
                verify_hash(&hash, expected_hash)?;
                Some(hash)
            } else {
                None
            };

            if let Some(modified) = util::parse_last_modified_header(&headers)? {
                file.set_modified(modified)?;
            }

            Ok(Response::new_save_to_file_response(
                size.try_into().unwrap(),
                hash,
                headers,
            ))
        } else {
            let bytes = response.bytes()?;
            let hash = if let Some(digest) = digest {
                digest.reset();
                digest.update(&bytes);
                let mut hash = vec![0; digest.output_size()];
                digest.finalize_into_reset(hash.as_mut()).unwrap();
                verify_hash(&hash, expected_hash)?;
                Some(hash)
            } else {
                None
            };

            Ok(Response::new_bytes_response(bytes, hash, headers))
        }
    }
}

// ======================================================================
// FUNCTIONS - PRIVATE
// ======================================================================

fn verify_hash(got: &[u8], expected: &Option<String>) -> Result<(), Error> {
    if let Some(expected) = expected {
        let got = hex::encode(got);
        if got != *expected {
            return Err(Error::HashMismatch {
                got,
                expected: expected.clone(),
            });
        }
    }
    Ok(())
}
