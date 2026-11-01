//! Currency rate fetching and cache refresh (spec: doc 04 §3.2, doc 08 §1/§3;
//! decisions D16/D18).
//!
//! Provider downloads use `https://exchange-api.gnome.org/<provider>` (D16).
//! Refresh semantics (D18, from doc 08 §1/§3): `refresh-interval` 0 means
//! never fetch (cache only); otherwise a provider is fetched when its cache
//! file is missing or older than the interval, and a *forced* refresh (the
//! UNKNOWN_RATE retry in CLI mode, the "Refresh rates" link) fetches
//! regardless of age. Any failure falls back to the cached file, and
//! offline behavior is simply "whatever the cache holds".

use crate::currency::{parse_boc, parse_ecb, parse_imf, parse_unt, CurrencyTable};
use crate::persistence::{provider_cache_path, store_provider_cache, Provider};
use std::io::Read;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Download size cap per provider file (defensive; provider files are
/// well under this).
const MAX_DOWNLOAD_BYTES: u64 = 16 * 1024 * 1024;

const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

pub fn provider_url(p: Provider) -> &'static str {
    match p {
        Provider::Ims => "https://exchange-api.gnome.org/imf",
        Provider::Unt => "https://exchange-api.gnome.org/unt",
        Provider::Ecb => "https://exchange-api.gnome.org/ecb",
        Provider::Boc => "https://exchange-api.gnome.org/boc",
    }
}

/// Fetch one provider's file over the network (size-capped).
pub fn fetch_provider(p: Provider) -> Result<String, String> {
    let resp = ureq::get(provider_url(p))
        .timeout(FETCH_TIMEOUT)
        .call()
        .map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    resp.into_reader()
        .take(MAX_DOWNLOAD_BYTES)
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;
    String::from_utf8(buf).map_err(|_| "provider response is not valid UTF-8".to_string())
}

/// File age in seconds (None when missing or the clock is odd).
fn cache_age_secs(path: &std::path::Path) -> Option<i64> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let then = mtime.duration_since(UNIX_EPOCH).ok()?;
    Some((now - then).as_secs() as i64)
}

fn parse_provider(p: Provider, text: &str, table: &mut CurrencyTable) {
    match p {
        Provider::Ims => parse_imf(text, table),
        Provider::Unt => parse_unt(text, table),
        Provider::Ecb => parse_ecb(text, table),
        Provider::Boc => parse_boc(text, table),
    }
}

/// What triggers a refresh (doc 08 §1/§3).
#[derive(Debug, Clone, Copy)]
pub struct RefreshOptions {
    /// `refresh-interval` setting; 0 = never fetch online.
    pub interval_secs: i64,
    /// Force a fetch even when cached data is fresh (UNKNOWN_RATE retry,
    /// "Refresh rates" link).
    pub force: bool,
}

impl RefreshOptions {
    pub fn startup(interval_secs: i64) -> Self {
        RefreshOptions {
            interval_secs,
            force: false,
        }
    }
}

/// Build the rate table: for each provider in attempt order (IMF, UN, ECB,
/// BOC), fetch or reuse the cache per [`RefreshOptions`], then parse.
/// First provider to set a currency wins; later providers fill only
/// currencies that still lack rates.
pub fn refresh_rates(opts: RefreshOptions) -> CurrencyTable {
    let mut table = CurrencyTable::new();
    for p in Provider::ALL {
        let path = provider_cache_path(p);
        let cached = std::fs::read_to_string(&path).ok();
        let text = if opts.interval_secs == 0 {
            cached
        } else {
            let stale = match cache_age_secs(&path) {
                None => true,
                Some(age) => age > opts.interval_secs,
            };
            if opts.force || stale {
                match fetch_provider(p) {
                    Ok(t) => {
                        // Cache before parsing so a crash cannot lose the
                        // fresh download.
                        let _ = store_provider_cache(p, &t);
                        Some(t)
                    }
                    Err(_) => cached,
                }
            } else {
                cached
            }
        };
        if let Some(t) = text {
            parse_provider(p, &t, &mut table);
        }
    }
    table
}

/// Parse cached files only (no network) — startup fallback for offline runs.
pub fn load_cached_rates() -> CurrencyTable {
    let mut table = CurrencyTable::new();
    for p in Provider::ALL {
        if let Ok(text) = std::fs::read_to_string(provider_cache_path(p)) {
            parse_provider(p, &text, &mut table);
        }
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_urls_match_d16() {
        assert_eq!(
            provider_url(Provider::Ecb),
            "https://exchange-api.gnome.org/ecb"
        );
        assert_eq!(
            provider_url(Provider::Ims),
            "https://exchange-api.gnome.org/imf"
        );
        assert_eq!(
            provider_url(Provider::Unt),
            "https://exchange-api.gnome.org/unt"
        );
        assert_eq!(
            provider_url(Provider::Boc),
            "https://exchange-api.gnome.org/boc"
        );
    }
}
