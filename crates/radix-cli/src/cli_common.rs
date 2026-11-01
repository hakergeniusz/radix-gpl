//! Shared CLI plumbing (spec: doc 08 §4–5).
//!
//! Input normalization: the locale thousands separator is removed (unless
//! it is `.`) and the locale radix is replaced by `.` (doc 08 §4). Radix
//! ships the C locale (D14): thousands `,`, radix `.`.

use radix_core::currency_fetch::RefreshOptions;
use radix_core::error::{Error, ErrorCode};
use radix_core::eval::{EvalConfig, EvalState};
use radix_core::numfuncs::AngleUnit;
use radix_core::serializer::{NumberFormat, Serializer};
use radix_core::solve::solve;
use radix_core::units::UnitCatalog;

/// CLI evaluation context (doc 08 §4): base 10, word length 32,
/// library-default precision 1000 (differs from the GUI's 2000 — long
/// computations may differ in the last digits, per the doc's note).
pub fn cli_config(angle: AngleUnit) -> EvalConfig {
    EvalConfig {
        precision: 1000,
        wordlen: 32,
        angle_unit: angle,
        base: 10,
    }
}

/// Locale normalization for CLI input (doc 08 §4).
pub fn normalize_input(input: &str) -> String {
    let thousands = ',';
    let radix = '.'; // C locale radix is already the canonical form
    input
        .chars()
        .filter(|c| *c != thousands)
        .collect::<String>()
        .replace(radix, ".")
}

/// Solve one normalized expression end-to-end: rates from the cache (no
/// network), with the doc's UNKNOWN_RATE force-refresh retry.
pub fn solve_cli(
    canonical: &str,
    angle: AngleUnit,
    state: &mut EvalState,
) -> Result<String, Error> {
    let config = cli_config(angle);
    let units = UnitCatalog::new();
    // Cached rates only (no network); a miss triggers the doc's force refresh.
    let currencies = radix_core::currency_fetch::load_cached_rates();
    let out = solve(canonical, &config, state, &units, &currencies, None);
    if let Some(e) = &out.error {
        if e.code == ErrorCode::UnknownRate {
            // Force-refresh (sync) and retry once (doc 08 §4).
            let refreshed = radix_core::currency_fetch::refresh_rates(RefreshOptions {
                interval_secs: 604_800,
                force: true,
            });
            let out = solve(canonical, &config, state, &units, &refreshed, None);
            return finish(out);
        }
        return Err(e.clone());
    }
    finish(out)
}

fn finish(out: radix_core::solve::SolveOutcome) -> Result<String, Error> {
    if let Some(e) = out.error {
        return Err(e);
    }
    let v = out.value.ok_or_else(|| Error::mp("No sane value"))?;
    let mut ser = Serializer {
        format: NumberFormat::Automatic,
        accuracy: 9,
        show_thousands: false,
        ..Serializer::default()
    };
    // Base conversions override the display base (`10 in hex` → `A₁₆`).
    ser.repr_base = if out.repr_base == 0 {
        10
    } else {
        out.repr_base
    };
    ser.serialize(&v)
}

/// Print an error per the `radix-cmd` contract (doc 08 §5): MP →
/// `Error <message>`, others → `Error <numeric-code>` (D19).
pub fn print_error(e: &Error) {
    if e.code == ErrorCode::Mp {
        eprintln!("Error {}", e.message);
    } else {
        eprintln!("Error {}", e.code.code_num());
    }
}
