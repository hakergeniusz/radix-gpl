//! `radix-cmd` — console calculator (spec: doc 08 §5, man page gcalccmd(1)).
//!
//! With an argument: solve it (base 10, word length 32, always degrees)
//! and exit. Without: a REPL with a `> ` prompt; `exit`, `quit`, an empty
//! line, or EOF exits. Results go to stdout (AUTOMATIC format, accuracy 9);
//! errors go to stderr as `Error <message>` (MP) or `Error <numeric-code>`.

mod cli_common;

use cli_common::{normalize_input, print_error, solve_cli};
use radix_core::currency_fetch::RefreshOptions;
use radix_core::eval::EvalState;
use radix_core::numfuncs::AngleUnit;
use radix_core::settings::{KeyfileStore, SettingsStore};
use std::io::{BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if let Some(a) = args.first() {
        if a == "--help" || a == "-h" {
            println!("Usage: radix-cmd [EXPRESSION]");
            return;
        }
    }

    // Currency is loaded synchronously at startup, honoring
    // refresh-interval (doc 08 §5).
    let interval = KeyfileStore::new().load().refresh_interval;
    let _ = radix_core::currency_fetch::refresh_rates(RefreshOptions::startup(interval));

    if !args.is_empty() {
        // Multiple arguments are joined into one expression.
        solve_and_print(&normalize_input(&args.join(" ")));
        return;
    }

    repl();
}

fn solve_and_print(normalized: &str) {
    let mut state = EvalState::default();
    // gcalccmd is always degrees (doc 08 §5).
    match solve_cli(normalized, AngleUnit::Degrees, &mut state) {
        Ok(s) => println!("{s}"),
        Err(e) => {
            print_error(&e);
            std::process::exit(1);
        }
    }
}

fn repl() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut state = EvalState::default();
    let mut lines = stdin.lock().lines();
    loop {
        print!("> ");
        let _ = out.flush();
        let Some(line) = lines.next() else {
            // EOF: no further prompt is printed.
            return;
        };
        let line = match line {
            Ok(l) => l,
            Err(_) => return,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "exit" || trimmed == "quit" {
            return;
        }
        match solve_cli(&normalize_input(trimmed), AngleUnit::Degrees, &mut state) {
            Ok(s) => println!("{s}"),
            Err(e) => print_error(&e),
        }
    }
}
