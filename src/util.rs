use std::time::{Duration, SystemTime};

use reqwest::header::HeaderMap;

use crate::Error;

// ======================================================================
// FUNCTIONS - CRATE
// ======================================================================

pub(crate) fn parse_last_modified_header(headers: &HeaderMap) -> Result<Option<SystemTime>, Error> {
    if let Some(mtime) = headers.get(reqwest::header::LAST_MODIFIED) {
        let mtime = mtime
            .to_str()
            .map_err(|_| Error::InvalidLastModifiedHeader)?;

        let mtime =
            httpdate::parse_http_date(mtime).map_err(|_| Error::InvalidLastModifiedHeader)?;

        Ok(Some(mtime))
    } else {
        Ok(None)
    }
}

pub(crate) fn random_duration(min: Duration, max: Duration) -> Duration {
    Duration::from_micros(fastrand::u64(
        min.as_micros() as u64..=max.as_micros() as u64,
    ))
}
