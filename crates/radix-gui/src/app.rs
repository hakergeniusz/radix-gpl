//! Application shell (doc 06 §1): multiple independent windows, startup
//! settings, global actions.

use crate::win::CalcWindow;
use gtk::gio;
use gtk::prelude::*;
use radix_core::settings::ButtonMode;
use std::cell::RefCell;

thread_local! {
    static WINDOW_COUNT: RefCell<u32> = const { RefCell::new(0) };
}

/// A window went away — keeps the Ctrl+Q "Close All?" check honest when
/// windows are opened and closed through the menu (doc 06 §3).
pub(crate) fn window_closed() {
    WINDOW_COUNT.with(|c| {
        let n = c.borrow().saturating_sub(1);
        *c.borrow_mut() = n;
    });
}

pub fn run(equation: Option<String>, mode: Option<ButtonMode>) -> std::process::ExitCode {
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
        let _win = CalcWindow::new(app.upcast_ref(), equation.clone(), mode);
        WINDOW_COUNT.with(|c| *c.borrow_mut() += 1);
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
            WINDOW_COUNT.with(|c| *c.borrow_mut() += 1);
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

/// Ctrl+Q confirmation dialog lives on each window (win.close-all).
pub fn bind_close_all(win: &adw::ApplicationWindow) {
    let action = gio::SimpleAction::new("close-all", None);
    action.connect_activate(move |_, _| {
        let many = WINDOW_COUNT.with(|c| *c.borrow() > 1);
        if !many {
            std::process::exit(0);
        }
    });
    win.add_action(&action);
}
