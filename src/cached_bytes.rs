use std::time::{Duration, Instant};
use std::fmt;
use std::error;

use bytes::Bytes;
use reqwest::StatusCode;

type CacheUpdateFn = fn (&str) -> Result<Bytes, Box<dyn error::Error>>;

#[derive(Debug)]
pub enum MartaError {
    Unauthorized,
    InternalServerError,
    GenericError(StatusCode)
}

impl fmt::Display for MartaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            MartaError::Unauthorized => write!(f, "{}", "Authorization failed against MARTA API"),
            MartaError::InternalServerError => write!(f, "{}", "MARTA API did not return results"),
            MartaError::GenericError(code) => write!(f, "MARTA API returned HTTP {}", code)
        }
    }
}

impl error::Error for MartaError {}

pub struct CachedBytes {
    expires_at: Instant,
    ttl:  Duration,
    url:  String,
    data: Bytes,
    updater: CacheUpdateFn
}


impl CachedBytes {
    pub fn new<S: Into<String>>(url: S, ttl: Duration) -> CachedBytes {
        CachedBytes::new_with_updater(url, ttl, |endpoint| {
            let mut resp = reqwest::get(endpoint)?;
            let status = resp.status();

            match status {
                StatusCode::OK => {
                    let text = resp.text()?;
                    Ok(Bytes::from(text))
                },
                StatusCode::UNAUTHORIZED => Err(Box::new(MartaError::Unauthorized)),
                StatusCode::INTERNAL_SERVER_ERROR => Err(Box::new(MartaError::InternalServerError)),
                _ => Err(Box::new(MartaError::GenericError(status)))
            }
        })
    }

    pub fn new_with_updater<S: Into<String>>(url: S, ttl: Duration, updater: CacheUpdateFn) -> CachedBytes {
        CachedBytes {
            expires_at: Instant::now(),
            ttl:  ttl,
            url:  url.into(),
            data: Bytes::from(&b"<unused>"[..]),
            updater: updater
        }
    }

    pub fn is_valid(&self) -> bool {
        self.expires_at > Instant::now()
    }

    pub fn refresh(&mut self) -> Result<(), Box<dyn error::Error>> {
        self.data = (self.updater)(&self.url)?;
        self.expires_at = Instant::now() + self.ttl;
        Ok(())
    }

    pub fn bytes(&self) -> Bytes {
        self.data.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_not_valid_initially() {
        let cache = CachedBytes::new("https://example.test",
                                     Duration::from_secs(10));

        assert_eq!(cache.is_valid(), false);
    }

    #[test]
    fn is_valid_after_refresh() {
        let mut cache = CachedBytes::new_with_updater("https://example.test",
                                                      Duration::from_secs(10),
                                                      |_url| { Ok(Bytes::new()) });

        assert_eq!(cache.is_valid(), false);

        assert_eq!(cache.refresh().is_ok(), true);

        assert_eq!(cache.is_valid(), true);
    }

    #[test]
    fn is_invalid_after_cache_time_elapses() {
        let mut cache = CachedBytes::new_with_updater("https://example.test",
                                                      Duration::from_millis(10),
                                                      |_url| { Ok(Bytes::new()) });
        cache.refresh().unwrap();

        assert_eq!(cache.is_valid(), true);

        std::thread::sleep(Duration::from_millis(10));

        assert_eq!(cache.is_valid(), false);
    }

    #[test]
    fn is_invalid_if_refresh_fails() {
        let mut cache = CachedBytes::new_with_updater("https://example.test",
                                                      Duration::from_millis(10),
                                                      |_url| { Err(Box::new(MartaError::InternalServerError)) });

        assert_eq!(cache.refresh().is_err(), true);
        assert_eq!(cache.is_valid(), false);
    }
}
