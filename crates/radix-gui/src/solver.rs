//! Asynchronous solve pipeline (doc 05 §1.3): solves run on a worker
//! thread; the UI polls the result channel every 50 ms and shows a spinner
//! ("Calculating") after 100 ms.

use radix_core::currency::CurrencyTable;
use radix_core::currency_fetch::{refresh_rates, RefreshOptions};
use radix_core::eval::{EvalConfig, EvalState};
use radix_core::solve::{solve, SolveOutcome};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

/// Everything the UI needs when a solve completes.
pub struct SolveReply {
    pub outcome: SolveOutcome,
    /// The mutated evaluation state (variables/functions/answer).
    pub new_state: EvalState,
}

/// Kick off a solve on a worker thread; poll the returned receiver.
/// The evaluation state mutex is held for the duration of the solve —
/// the UI ignores input while a solve is pending (doc 05 §1.3).
pub fn spawn_solve(
    canonical: &str,
    config: EvalConfig,
    state: &Arc<Mutex<EvalState>>,
    currencies: &Arc<Mutex<CurrencyTable>>,
) -> Receiver<SolveReply> {
    let canonical = canonical.to_string();
    let state = Arc::clone(state);
    let currencies = Arc::clone(currencies);
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let reply = {
            let mut st = state.lock().expect("eval state poisoned");
            let currencies = currencies.lock().expect("currency table poisoned");
            let outcome = solve(&canonical, &config, &mut st, units(), &currencies, None);
            SolveReply {
                outcome,
                new_state: st.clone(),
            }
        };
        let _ = tx.send(reply);
    });
    rx
}

/// Background currency refresh (doc 08 §3): started at window open and
/// when refresh-interval changes; result lands in the shared table.
pub fn spawn_rate_refresh(
    interval_secs: i64,
    currencies: &Arc<Mutex<CurrencyTable>>,
) -> Receiver<CurrencyTable> {
    let currencies = Arc::clone(currencies);
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let table = refresh_rates(RefreshOptions::startup(interval_secs));
        if let Ok(mut guard) = currencies.lock() {
            if !table.rates.is_empty() {
                *guard = table.clone();
            }
        }
        let _ = tx.send(table);
    });
    rx
}

/// Forced currency refresh (converter panel "Refresh rates" link): fetches
/// regardless of the cache age and replaces the table when successful.
pub fn spawn_rate_refresh_forced(
    currencies: &Arc<Mutex<CurrencyTable>>,
) -> Receiver<CurrencyTable> {
    let currencies = Arc::clone(currencies);
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let table = refresh_rates(RefreshOptions {
            interval_secs: 0,
            force: true,
        });
        if let Ok(mut guard) = currencies.lock() {
            if !table.rates.is_empty() {
                *guard = table.clone();
            }
        }
        let _ = tx.send(table);
    });
    rx
}

fn units() -> &'static UnitCatalog {
    use std::sync::OnceLock;
    static UNITS: OnceLock<UnitCatalog> = OnceLock::new();
    UNITS.get_or_init(UnitCatalog::new)
}

/// Poll helper: drains one reply if present.
pub fn poll(rx: &Receiver<SolveReply>) -> Option<SolveReply> {
    rx.try_recv().ok()
}

use radix_core::units::UnitCatalog;
