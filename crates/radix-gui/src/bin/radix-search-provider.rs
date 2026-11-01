//! `radix-search-provider` — GNOME Shell search provider (doc 08 §6).
//!
//! Owns `io.github.hakergeniusz.Radix.SearchProvider` on the session bus and serves
//! `org.gnome.Shell.SearchProvider2` at `/io/github/hakergeniusz/Radix/SearchProvider`
//! (activation via the installed D-Bus service file). At most one result per
//! query — the candidate expression plus a companion "Open in Calculator"
//! entry. Queries are rejected without results when the normalized text
//! parses as a plain number, contains `=` (variables/functions are never
//! mutated from search), fails to evaluate, or hits the 50 ms evaluation
//! budget. The last 10 equations are cached for the service lifetime and the
//! service idles out after 10 s of inactivity.

#[path = "../cli.rs"]
mod cli;

use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use radix_core::currency_fetch::load_cached_rates;
use radix_core::eval::{Budget, EvalConfig, EvalState};
use radix_core::numfuncs::AngleUnit;
use radix_core::serializer::{NumberFormat, Serializer};
use radix_core::settings::{AngleSetting, KeyfileStore, SettingsStore};
use radix_core::solve::solve;
use radix_core::units::UnitCatalog;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;

const SERVICE_NAME: &str = "io.github.hakergeniusz.Radix.SearchProvider";
const OBJECT_PATH: &str = "/io/github/hakergeniusz/Radix/SearchProvider";
const INTERFACE: &str = "org.gnome.Shell.SearchProvider2";
const OPEN_PREFIX: &str = "open-in-calculator-";
const APP_ICON: &str = "io.github.hakergeniusz.Radix";
const COPY_ICON: &str = "edit-copy-symbolic";
const CACHE_LIMIT: usize = 10;
const IDLE_EXIT: Duration = Duration::from_secs(10);

const INTROSPECTION: &str = r#"<node>
  <interface name="org.gnome.Shell.SearchProvider2">
    <method name="GetInitialResultSet">
      <arg type="as" direction="in" name="terms"/>
      <arg type="as" direction="out" name="results"/>
    </method>
    <method name="GetSubsearchResultSet">
      <arg type="as" direction="in" name="previous_results"/>
      <arg type="as" direction="in" name="terms"/>
      <arg type="as" direction="out" name="results"/>
    </method>
    <method name="GetResultMetas">
      <arg type="as" direction="in" name="identifiers"/>
      <arg type="aa{sv}" direction="out" name="results"/>
    </method>
    <method name="ActivateResult">
      <arg type="s" direction="in" name="identifier"/>
      <arg type="as" direction="in" name="terms"/>
      <arg type="u" direction="in" name="timestamp"/>
    </method>
    <method name="LaunchSearch">
      <arg type="as" direction="in" name="terms"/>
      <arg type="u" direction="in" name="timestamp"/>
    </method>
  </interface>
</node>"#;

struct Provider {
    /// (equation, result) pairs, oldest first, capped at CACHE_LIMIT.
    cache: RefCell<VecDeque<(String, String)>>,
    /// Reset on every D-Bus call; the idle timer exits when it ages past 10 s.
    last_activity: Cell<Instant>,
    /// Clipboard for activation, present only when a display could be opened.
    clipboard: Option<gdk::Clipboard>,
    /// Session-bus connection (notification proxy source), set on ownership.
    connection: RefCell<Option<gio::DBusConnection>>,
    /// Set from main() so the idle timer can stop the service.
    main_loop: RefCell<Option<glib::MainLoop>>,
}

impl Provider {
    fn touch(&self) {
        self.last_activity.set(Instant::now());
    }

    /// GetInitialResultSet: evaluate the joined terms and return at most the
    /// pair [equation, "open-in-calculator-" + equation] (doc 08 §6).
    fn initial(&self, terms: &[String]) -> Vec<String> {
        let equation = terms.join(" ");
        match evaluate_query(&equation, angle_from_settings()) {
            Some(result) => {
                self.remember(equation.clone(), result);
                vec![equation.clone(), format!("{OPEN_PREFIX}{equation}")]
            }
            None => Vec::new(),
        }
    }

    /// GetSubsearchResultSet: prefer a fresh evaluation of the narrowed
    /// terms; failing that, narrow the cached equations by substring.
    fn subsearch(&self, previous: &[String], terms: &[String]) -> Vec<String> {
        let fresh = self.initial(terms);
        if !fresh.is_empty() {
            return fresh;
        }
        let needle = terms.join(" ").to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<String> = Vec::new();
        // The open-in identifiers embed the full equation, so they alone
        // cover every previous result without duplication.
        for id in previous {
            let Some(eq) = id.strip_prefix(OPEN_PREFIX) else {
                continue;
            };
            if !eq.to_lowercase().contains(&needle) {
                continue;
            }
            for candidate in [eq.to_string(), format!("{OPEN_PREFIX}{eq}")] {
                if !out.contains(&candidate) {
                    out.push(candidate);
                }
            }
        }
        out
    }

    /// GetResultMetas: the equation entry (name = equation, description
    /// " = <result>", copy icon, clipboardText = result) and the
    /// "Open in Calculator" companion (app icon).
    fn metas(&self, ids: &[String]) -> Vec<HashMap<String, glib::Variant>> {
        ids.iter()
            .map(|id| {
                let mut m: HashMap<String, glib::Variant> = HashMap::new();
                m.insert("id".into(), id.to_variant());
                if let Some(eq) = id.strip_prefix(OPEN_PREFIX) {
                    let _ = eq;
                    m.insert("name".into(), "Open in Calculator".to_variant());
                    m.insert("icon".into(), APP_ICON.to_variant());
                    return m;
                }
                m.insert("name".into(), id.to_variant());
                if let Some(result) = self
                    .cached_result(id)
                    .or_else(|| evaluate_query(id, angle_from_settings()))
                {
                    m.insert("description".into(), format!(" = {result}").to_variant());
                    m.insert("clipboardText".into(), result.to_variant());
                }
                m.insert("icon".into(), COPY_ICON.to_variant());
                m
            })
            .collect()
    }

    /// ActivateResult: the open-in entry spawns the calculator with the
    /// expression loaded; the result entry copies the result to the
    /// clipboard and shows a transient notification (doc 08 §6).
    fn activate(self: &Rc<Self>, id: &str, terms: &[String]) {
        if let Some(eq) = id.strip_prefix(OPEN_PREFIX) {
            let equation = if eq.is_empty() {
                terms.join(" ")
            } else {
                eq.to_string()
            };
            self.spawn_calculator(&equation);
            return;
        }
        let Some(result) = self.cached_result(id) else {
            return;
        };
        if let Some(clipboard) = &self.clipboard {
            clipboard.set_text(&result);
        }
        self.notify_copied();
    }

    /// LaunchSearch: open the calculator with the search terms (doc 08 §6).
    fn launch(&self, terms: &[String]) {
        let equation = terms.join(" ");
        if !equation.trim().is_empty() {
            self.spawn_calculator(&equation);
        }
    }

    /// Spawn `radix --equation <equation>`; the sibling of this executable
    /// is preferred so a build tree works without installation.
    fn spawn_calculator(&self, equation: &str) {
        let exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("radix")))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "radix".to_string());
        let cmdline = format!("{} --equation {}", exe, shell_quote(equation));
        if let Err(e) = glib::spawn_command_line_async(&cmdline) {
            eprintln!("radix-search-provider: failed to launch calculator: {e}");
        }
    }

    /// "Result copied" / "Result was copied successfully", withdrawn after
    /// 4 s (doc 08 §6). Requires the session bus; silently skipped headless.
    fn notify_copied(self: &Rc<Self>) {
        let Some(conn) = self.connection.borrow().clone() else {
            return;
        };
        let proxy = match gio::DBusProxy::new_sync(
            &conn,
            gio::DBusProxyFlags::NONE,
            None,
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            gio::Cancellable::NONE,
        ) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("radix-search-provider: notification proxy failed: {e}");
                return;
            }
        };
        let params = (
            "Calculator",
            0u32,
            APP_ICON,
            "Result copied",
            "Result was copied successfully",
            Vec::<String>::new(),
            HashMap::<String, glib::Variant>::new(),
            -1i32,
        )
            .to_variant();
        let proxy_close = proxy.clone();
        proxy.call(
            "Notify",
            Some(&params),
            gio::DBusCallFlags::NONE,
            1_000,
            gio::Cancellable::NONE,
            move |res: Result<glib::Variant, glib::Error>| {
                let Ok(reply) = res else {
                    eprintln!("radix-search-provider: Notify failed: {}", res.unwrap_err());
                    return;
                };
                let id = reply.child_value(0).get::<u32>().unwrap_or(0);
                if id == 0 {
                    return;
                }
                glib::timeout_add_local_once(Duration::from_secs(4), move || {
                    proxy_close.call(
                        "CloseNotification",
                        Some(&(id,).to_variant()),
                        gio::DBusCallFlags::NONE,
                        1_000,
                        gio::Cancellable::NONE,
                        |_| {},
                    );
                });
            },
        );
    }

    fn remember(&self, equation: String, result: String) {
        let mut cache = self.cache.borrow_mut();
        cache.retain(|(e, _)| *e != equation);
        cache.push_back((equation, result));
        while cache.len() > CACHE_LIMIT {
            cache.pop_front();
        }
    }

    fn cached_result(&self, equation: &str) -> Option<String> {
        self.cache
            .borrow()
            .iter()
            .find(|(e, _)| e == equation)
            .map(|(_, r)| r.clone())
    }
}

/// Plain-number check (doc 08 §6): optional sign, digits, one decimal point.
fn is_plain_number(text: &str) -> bool {
    let trimmed = text.trim();
    let body = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
    match body.split_once('.') {
        Some((int_part, frac)) => {
            (!int_part.is_empty() || !frac.is_empty())
                && int_part.chars().all(|c| c.is_ascii_digit())
                && frac.chars().all(|c| c.is_ascii_digit())
        }
        None => !body.is_empty() && body.chars().all(|c| c.is_ascii_digit()),
    }
}

/// The candidate evaluation pipeline (doc 08 §6): normalize, reject plain
/// numbers and assignments, evaluate under the 50 ms budget (base 10,
/// wordlen 32, angle units from settings), render AUTOMATIC accuracy 9.
/// `None` means the query must not produce a search result.
fn evaluate_query(query: &str, angle: AngleUnit) -> Option<String> {
    if query.trim().is_empty() || query.contains('=') {
        return None;
    }
    let normalized = cli::normalize_input(query);
    if is_plain_number(&normalized) {
        return None;
    }
    let config = EvalConfig {
        precision: 1000, // CLI/search precision (decision D5)
        wordlen: 32,
        angle_unit: angle,
        base: 10,
    };
    let mut budget = Budget::new(); // 50 ms deadline + doc 05 §5 limits
    let mut state = EvalState::default();
    let units = UnitCatalog::new();
    let currencies = load_cached_rates();
    let out = solve(
        &normalized,
        &config,
        &mut state,
        &units,
        &currencies,
        Some(&mut budget),
    );
    if out.error.is_some() {
        return None;
    }
    let value = out.value?;
    let mut ser = Serializer {
        format: NumberFormat::Automatic,
        accuracy: 9,
        show_thousands: false,
        ..Serializer::default()
    };
    // Base conversions override the display base (`10 in bin` → `1010₂`).
    ser.repr_base = if out.repr_base == 0 {
        config.base
    } else {
        out.repr_base
    };
    ser.serialize(&value).ok()
}

fn angle_from_settings() -> AngleUnit {
    match KeyfileStore::new().load().angle_units {
        AngleSetting::Radians => AngleUnit::Radians,
        AngleSetting::Gradians => AngleUnit::Gradians,
        AngleSetting::Degrees => AngleUnit::Degrees,
    }
}

/// Single-quote quoting for spawn_command_line_async (g_shell_parse_argv).
fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':'))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', r"'\''"))
}

fn main() -> std::process::ExitCode {
    // Internal/testing CLI (doc 08 §6): --solve / --angle-units exercise the
    // search evaluation pipeline without the bus.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut angle_override: Option<AngleUnit> = None;
    let mut solve_eq: Option<String> = None;
    let mut it = args.iter().peekable();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-s" | "--solve" => match it.next() {
                Some(v) => solve_eq = Some(v.clone()),
                None => {
                    eprintln!("radix-search-provider: --solve requires an argument");
                    return std::process::ExitCode::from(2);
                }
            },
            "--angle-units" => match it.next().map(|v| v.to_lowercase()) {
                Some(v) => match v.as_str() {
                    "degrees" | "deg" | "d" => angle_override = Some(AngleUnit::Degrees),
                    "radians" | "rad" | "r" => angle_override = Some(AngleUnit::Radians),
                    "gradians" | "grad" | "g" => angle_override = Some(AngleUnit::Gradians),
                    _ => {
                        eprintln!(
                            "radix-search-provider: unknown angle units {v:?} (degrees, radians, gradians)"
                        );
                        return std::process::ExitCode::from(2);
                    }
                },
                None => {
                    eprintln!("radix-search-provider: --angle-units requires an argument");
                    return std::process::ExitCode::from(2);
                }
            },
            "-v" | "--version" => {
                eprintln!("Radix search provider {}", env!("CARGO_PKG_VERSION"));
                return std::process::ExitCode::from(0);
            }
            "-h" | "--help" => {
                println!(
                    "Usage: radix-search-provider [OPTION…]\n\n\
                     -s, --solve EQUATION      Evaluate EQUATION with the search pipeline\n\
                         --angle-units UNITS   Angle units for --solve (degrees, radians, gradians)\n\
                     -v, --version             Print version information\n\
                     -h, --help                Print this help\n\n\
                     Without options the service owns {SERVICE_NAME} and idles\n\
                     out after {} s of inactivity.",
                    IDLE_EXIT.as_secs()
                );
                return std::process::ExitCode::from(0);
            }
            other => {
                eprintln!("radix-search-provider: unrecognized option {other:?}");
                return std::process::ExitCode::from(2);
            }
        }
    }
    if let Some(equation) = solve_eq {
        let angle = angle_override.unwrap_or_else(angle_from_settings);
        return match evaluate_query(&equation, angle) {
            Some(result) => {
                println!("{result}");
                std::process::ExitCode::from(0)
            }
            None => std::process::ExitCode::from(1),
        };
    }

    // A display is only needed for the clipboard on activation; without one
    // the service still runs (headless testing, doc 08 §7).
    let clipboard = match gtk::init() {
        Ok(()) => gdk::Display::default().map(|d| d.clipboard()),
        Err(_) => None,
    };

    let provider = Rc::new(Provider {
        cache: RefCell::new(VecDeque::new()),
        last_activity: Cell::new(Instant::now()),
        clipboard,
        connection: RefCell::new(None),
        main_loop: RefCell::new(None),
    });

    let loop_ = glib::MainLoop::new(None, false);
    *provider.main_loop.borrow_mut() = Some(loop_.clone());

    // Idle exit: quit when the last D-Bus call ages past the idle timeout.
    glib::timeout_add_local(IDLE_EXIT, {
        let provider = provider.clone();
        move || {
            if provider.last_activity.get().elapsed() >= IDLE_EXIT {
                if let Some(loop_) = provider.main_loop.borrow().as_ref() {
                    loop_.quit();
                }
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        }
    });

    let _owner = gio::bus_own_name(
        gio::BusType::Session,
        SERVICE_NAME,
        gio::BusNameOwnerFlags::empty(),
        {
            let provider = provider.clone();
            move |conn, _name| {
                *provider.connection.borrow_mut() = Some(conn.clone());
                let node = match gio::DBusNodeInfo::for_xml(INTROSPECTION) {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("radix-search-provider: introspection XML failed: {e}");
                        std::process::exit(1);
                    }
                };
                let Some(iface) = node.lookup_interface(INTERFACE) else {
                    eprintln!("radix-search-provider: interface missing from introspection");
                    std::process::exit(1);
                };
                let registration = conn.register_object(OBJECT_PATH, &iface).method_call({
                    let provider = provider.clone();
                    move |_conn, _sender, _path, _iface, method, params, invocation| {
                        provider.touch();
                        match method {
                            "GetInitialResultSet" => {
                                let terms = params
                                    .child_value(0)
                                    .get::<Vec<String>>()
                                    .unwrap_or_default();
                                let results = provider.initial(&terms);
                                invocation.return_value(Some(&(results,).to_variant()));
                            }
                            "GetSubsearchResultSet" => {
                                let previous = params
                                    .child_value(0)
                                    .get::<Vec<String>>()
                                    .unwrap_or_default();
                                let terms = params
                                    .child_value(1)
                                    .get::<Vec<String>>()
                                    .unwrap_or_default();
                                let results = provider.subsearch(&previous, &terms);
                                invocation.return_value(Some(&(results,).to_variant()));
                            }
                            "GetResultMetas" => {
                                let ids = params
                                    .child_value(0)
                                    .get::<Vec<String>>()
                                    .unwrap_or_default();
                                let metas = provider.metas(&ids);
                                invocation.return_value(Some(&(metas,).to_variant()));
                            }
                            "ActivateResult" => {
                                let id = params.child_value(0).get::<String>().unwrap_or_default();
                                let terms = params
                                    .child_value(1)
                                    .get::<Vec<String>>()
                                    .unwrap_or_default();
                                provider.activate(&id, &terms);
                                invocation.return_value(None);
                            }
                            "LaunchSearch" => {
                                let terms = params
                                    .child_value(0)
                                    .get::<Vec<String>>()
                                    .unwrap_or_default();
                                provider.launch(&terms);
                                invocation.return_value(None);
                            }
                            _ => {
                                invocation.return_dbus_error(
                                    "org.freedesktop.DBus.Error.UnknownMethod",
                                    "Unknown method",
                                );
                            }
                        }
                    }
                });
                if let Err(e) = registration.build() {
                    eprintln!("radix-search-provider: object registration failed: {e}");
                    std::process::exit(1);
                }
            }
        },
        |_, _| {},
        |_conn, name| {
            eprintln!("radix-search-provider: could not own {name} (another instance running?)");
            std::process::exit(1);
        },
    );

    loop_.run();
    std::process::ExitCode::from(0)
}
