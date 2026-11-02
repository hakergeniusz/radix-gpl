//! Application shell (doc 06 §1): multiple independent windows, startup
//! settings, global actions.

use crate::win::CalcWindow;
use gtk::gio;
use gtk::prelude::*;
use radix_core::settings::ButtonMode;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

thread_local! {
    /// Every live window, hidden ones included — closing a window moves it
    /// to the background (D31), so Quit iterates this registry rather than
    /// the application window list. Weak refs; dead entries are skipped.
    static WINDOWS: RefCell<Vec<Weak<CalcWindow>>> = const { RefCell::new(Vec::new()) };
}

/// Track a window so Ctrl+Q / menu Quit can persist every window's state
/// before exit (D31).
pub(crate) fn register_window(win: &Rc<CalcWindow>) {
    WINDOWS.with(|c| c.borrow_mut().push(Rc::downgrade(win)));
}

/// Save every live window's state, then exit (Ctrl+Q / menu Quit, D31).
fn quit_saving() {
    let windows: Vec<Rc<CalcWindow>> = WINDOWS
        .with(|c| c.borrow().iter().filter_map(|w| w.upgrade()).collect());
    for win in &windows {
        win.save_shutdown_state();
    }
    std::process::exit(0);
}

pub fn run(equation: Option<String>, mode: Option<ButtonMode>) -> std::process::ExitCode {
    // First-map renderer (D32): the autodetect path probes Vulkan before
    // falling back to GL, which costs ~120 ms at the first frame on the
    // startup harness (286 ms → 170 ms). ngl is the modern GNOME default
    // anyway; an explicit GSK_RENDERER from the environment still wins.
    if std::env::var_os("GSK_RENDERER").is_none() {
        std::env::set_var("GSK_RENDERER", "ngl");
    }

    // adw::Application initializes libadwaita and loads its stylesheet —
    // with plain gtk::Application the Adw widgets render unstyled (stock
    // GTK Adwaita, always light) regardless of the color scheme.
    let app = adw::Application::builder()
        .application_id("io.github.hakergeniusz.Radix")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    // App-level actions behind the primary menu (doc 06 §3, upstream
    // gnome-calculator.vala): new window, shortcuts, help, about.
    install_app_actions(&app);
    install_accels(&app);

    app.connect_activate(move |app| {
        // GNOME 51 reference header (doc 13): close button only.
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::Settings::for_display(&display).set_gtk_decoration_layout(Some(":close"));
        }
        let win = CalcWindow::new(app.upcast_ref(), equation.clone(), mode);
        register_window(&win);
    });

    // main.rs already parsed the command line itself; hand GTK an empty list
    // so its own GOption context doesn't reject our options (doc 08 §4).
    app.run_with_args(&[] as &[&str]);
    std::process::ExitCode::from(0)
}

fn install_app_actions(app: &adw::Application) {
    let new_window = gio::SimpleAction::new("new-window", None);
    {
        let app = app.clone();
        new_window.connect_activate(move |_, _| {
            let win = CalcWindow::new(app.upcast_ref(), None, None);
            win.window.present();
            register_window(&win);
        });
    }
    app.add_action(&new_window);

    let shortcuts = gio::SimpleAction::new("shortcuts", None);
    {
        let app = app.clone();
        shortcuts.connect_activate(move |_, _| {
            if let Some(win) = app.active_window() {
                crate::dialogs::open_shortcuts(&win);
            }
        });
    }
    app.add_action(&shortcuts);

    let help = gio::SimpleAction::new("help", None);
    {
        let app = app.clone();
        help.connect_activate(move |_, _| {
            // show_uri is the synchronous simple path; the 4.10+ replacement
            // is the async GtkUriLauncher.
            #[allow(deprecated)]
            gtk::show_uri(app.active_window().as_ref(), "help:radix", 0u32);
        });
    }
    app.add_action(&help);

    let about = gio::SimpleAction::new("about", None);
    {
        let app = app.clone();
        about.connect_activate(move |_, _| {
            if let Some(win) = app.active_window() {
                crate::dialogs::open_about(&win);
            }
        });
    }
    app.add_action(&about);
}

/// Accelerators for the menu actions (upstream gnome-calculator.vala): the
/// PopoverMenu displays them next to their items (D25).
fn install_accels(app: &adw::Application) {
    app.set_accels_for_action("win.mode::basic", &["<Ctrl><Alt>b"]);
    app.set_accels_for_action("win.mode::advanced", &["<Ctrl><Alt>a"]);
    app.set_accels_for_action("win.mode::financial", &["<Ctrl><Alt>f"]);
    app.set_accels_for_action("win.mode::programming", &["<Ctrl><Alt>p"]);
    app.set_accels_for_action("win.mode::keyboard", &["<Ctrl><Alt>k", "<Ctrl><Alt>t"]);
    app.set_accels_for_action("win.mode::conversion", &["<Ctrl><Alt>c"]);
    app.set_accels_for_action("win.clear", &["<Ctrl>Escape"]);
    app.set_accels_for_action("win.preferences", &["<Ctrl>comma"]);
    app.set_accels_for_action("app.new-window", &["<Ctrl>n"]);
    app.set_accels_for_action("app.shortcuts", &["<Ctrl>question"]);
    app.set_accels_for_action("app.help", &["F1"]);
    app.set_accels_for_action("window.close", &["<Ctrl>w"]);

    // Ctrl+Q with more than one window: "Close All?" confirmation
    // (doc 06 §3).
    app.set_accels_for_action("win.close-all", &["<Ctrl>q"]);
}

/// Ctrl+Q / menu Quit: persist every window's state and exit (D31). The
/// doc 06 §3 "Close All?" confirmation was never wired (the action used
/// to no-op for multiple windows); with close-as-background there is
/// nothing to confirm — Quit simply ends the app.
pub fn bind_close_all(win: &adw::ApplicationWindow) {
    let action = gio::SimpleAction::new("close-all", None);
    action.connect_activate(|_, _| quit_saving());
    win.add_action(&action);
}
