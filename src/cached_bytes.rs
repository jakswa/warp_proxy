use std::time::{Duration, Instant};
use bytes::Bytes;

pub struct CachedBytes {
    expires_at: Instant,
    cache_for:  Duration,
    url:  String,
    pub text: Bytes,
    pub updater: fn (&str) -> Bytes
}

impl CachedBytes {
    pub fn new<S: Into<String>>(url: S, cache_for: Duration) -> CachedBytes {
        CachedBytes::new_with_updater(url, cache_for, |endpoint| {
                let text = reqwest::get(endpoint).unwrap().text().unwrap();
                Bytes::from(text)
        })
    }

    pub fn new_with_updater<S: Into<String>>(url: S, cache_for: Duration, updater: fn (&str) -> Bytes) -> CachedBytes {
        CachedBytes {
            expires_at: Instant::now(),
            cache_for:  cache_for,
            url:  url.into(),
            text: Bytes::from(&b"<unused>"[..]),
            updater: updater
        }
    }

    pub fn is_valid(&self) -> bool {
        self.expires_at > Instant::now()
    }

    pub fn refresh(&mut self) {
        self.text = (self.updater)(&self.url);
        self.expires_at = Instant::now() + self.cache_for;
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
                                                      |_url| { Bytes::new() });

        assert_eq!(cache.is_valid(), false);

        cache.refresh();

        assert_eq!(cache.is_valid(), true);
    }

    #[test]
    fn is_invalid_after_cache_time_elapses() {
        let mut cache = CachedBytes::new_with_updater("https://example.test",
                                                      Duration::from_millis(10),
                                                      |_url| { Bytes::new() });
        cache.refresh();

        assert_eq!(cache.is_valid(), true);

        std::thread::sleep(Duration::from_millis(10));

        assert_eq!(cache.is_valid(), false);
    }

}
