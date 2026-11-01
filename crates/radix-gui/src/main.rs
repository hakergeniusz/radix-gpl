//! `radix` — GUI calculator binary (spec: doc 08 §4).
//!
//! CLI contract mirrors gnome-calculator: `--solve`/`-s` evaluates headless,
//! `--equation`/`-e` opens a window with text loaded, `--mode`/`-m` selects
//! the start mode, `--version`/`-v` prints to stderr and exits 0.

mod app;
mod cli;
mod converter;
mod dialogs;
mod display;
mod keypad;
mod popovers;
mod prefs;
mod solver;
mod win;

use radix_core::settings::ButtonMode;

fn parse_mode(s: &str) -> Option<ButtonMode> {
    Some(match s {
        "basic" => ButtonMode::Basic,
        "advanced" => ButtonMode::Advanced,
        "financial" => ButtonMode::Financial,
        "programming" => ButtonMode::Programming,
        "keyboard" => ButtonMode::Keyboard,
        "conversion" => ButtonMode::Conversion,
        _ => return None,
    })
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut solve: Option<String> = None;
    let mut equation: Option<String> = None;
    let mut mode: Option<ButtonMode> = None;

    let mut it = args.iter().peekable();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-s" | "--solve" => match it.next() {
                Some(v) => solve = Some(v.clone()),
                None => {
                    eprintln!("radix: --solve requires an argument");
                    return std::process::ExitCode::from(2);
                }
            },
            "-e" | "--equation" => {
                equation = it.next().map(|s| cli::unescape_argument(s));
            }
            "-m" | "--mode" => match it.next().map(|s| s.as_str()) {
                Some(m) => match parse_mode(m) {
                    Some(b) => mode = Some(b),
                    None => {
                        eprintln!("radix: unknown mode {m:?} (basic, advanced, financial, programming, keyboard, conversion)");
                        return std::process::ExitCode::from(2);
                    }
                },
                None => {
                    eprintln!("radix: --mode requires an argument");
                    return std::process::ExitCode::from(2);
                }
            },
            "-v" | "--version" => {
                // `<program-name> <version>` to stderr, exit 0 (doc 08 §4).
                eprintln!("Radix {}", env!("CARGO_PKG_VERSION"));
                return std::process::ExitCode::from(0);
            }
            "-h" | "--help" => {
                println!(
                    "Usage: radix [OPTION…]\n\n\
                     -s, --solve EQUATION     Evaluate EQUATION and exit\n\
                     -e, --equation EQUATION  Open a window with EQUATION loaded\n\
                     -m, --mode MODE          Start in MODE (basic, advanced, financial, programming, keyboard, conversion)\n\
                     -v, --version            Print version information\n\
                     -h, --help               Print this help"
                );
                return std::process::ExitCode::from(0);
            }
            other => {
                eprintln!("radix: unrecognized option {other:?}");
                return std::process::ExitCode::from(2);
            }
        }
    }

    if let Some(equation) = solve {
        // Headless path: no GTK init (doc 08 §4).
        return cli::run_solve(&equation);
    }

    app::run(equation, mode)
}
