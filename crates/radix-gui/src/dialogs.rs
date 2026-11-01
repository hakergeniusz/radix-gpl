//! Dialogs: financial dialogs (doc 04 §4), character dialog (doc 06 §5.3),
//! shortcuts window, and About (doc 06 §3).

use crate::win::CalcWindow;
use adw::prelude::*;
use gtk::Orientation;
use std::rc::Rc;

/// Financial dialog definitions (doc 04 §4): name and parameter labels in
/// order. The functions are dialog-only (doc 04 §4) — evaluation goes
/// through `radix_core::financial::evaluate`, not the expression catalog.
struct FinSpec {
    name: &'static str,
    params: &'static [&'static str],
}

const FIN_SPECS: &[FinSpec] = &[
    FinSpec {
        name: "Ctrm",
        params: &["Periodic Interest Rate", "Future Value", "Present Value"],
    },
    FinSpec {
        name: "Ddb",
        params: &["Cost", "Life", "Period"],
    },
    FinSpec {
        name: "Fv",
        params: &[
            "Periodic Payment",
            "Periodic Interest Rate",
            "Number of Periods",
        ],
    },
    FinSpec {
        name: "Gpm",
        params: &["Cost", "Margin"],
    },
    FinSpec {
        name: "Pmt",
        params: &["Principal", "Periodic Interest Rate", "Term"],
    },
    FinSpec {
        name: "Pv",
        params: &[
            "Periodic Payment",
            "Periodic Interest Rate",
            "Number of Periods",
        ],
    },
    FinSpec {
        name: "Rate",
        params: &["Future Value", "Present Value", "Number of Periods"],
    },
    FinSpec {
        name: "Sln",
        params: &["Cost", "Salvage Value", "Useful Life"],
    },
    FinSpec {
        name: "Syd",
        params: &["Cost", "Salvage Value", "Useful Life", "Period"],
    },
    FinSpec {
        name: "Term",
        params: &["Periodic Payment", "Future Value", "Periodic Interest Rate"],
    },
];

/// Financial parameter dialog (doc 04 §4; dialog chrome per doc 13 §8.3):
/// a headered Adw.Dialog — Cancel at the start, accent Calculate at the
/// end — with numeric entries. Calculate runs the dialog-only financial
/// function and **replaces the display** with the result (not added to
/// history); errors surface through the status line.
pub fn open_financial_dialog(win: &Rc<CalcWindow>, name: &'static str) {
    let Some(spec) = FIN_SPECS.iter().find(|s| s.name == name) else {
        return;
    };
    let dialog = adw::Dialog::new();
    dialog.set_title(name);

    let root = gtk::Box::new(Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);

    let prec = win.eval_config().precision;
    let entries: Vec<gtk::Entry> = spec
        .params
        .iter()
        .map(|p| {
            let e = gtk::Entry::new();
            e.set_placeholder_text(Some(p));
            // Entries accept only numbers (doc 04 §4).
            e.set_input_purpose(gtk::InputPurpose::Number);
            root.append(&e);
            e
        })
        .collect();

    // Headered presentation (doc 13 §8.3): header bar with title, Cancel
    // start, accent Calculate end.
    let header = adw::HeaderBar::new();
    let cancel_btn = gtk::Button::with_label("Cancel");
    cancel_btn.add_css_class("flat");
    header.pack_start(&cancel_btn);
    let solve_btn = gtk::Button::with_label("Calculate");
    solve_btn.add_css_class("suggested-action");
    header.pack_end(&solve_btn);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&root));

    dialog.set_child(Some(&toolbar));
    dialog.present(Some(win.window.upcast_ref::<gtk::Widget>()));

    cancel_btn.connect_clicked({
        let dlg = dialog.clone();
        move |_| {
            dlg.close();
        }
    });
    let win = Rc::clone(win);
    solve_btn.connect_clicked(move |_| {
        // Non-numeric content counts as 0 (doc 04 §4's entry semantics).
        let args: Vec<radix_core::Number> = entries
            .iter()
            .map(|e| {
                radix_core::number::parse_decimal_rational(e.text().trim())
                    .map(|r| radix_core::Number::rational(r, prec))
                    .unwrap_or_else(|| radix_core::Number::zero(prec))
            })
            .collect();
        match radix_core::financial::evaluate(spec.name, &args) {
            Ok(v) => {
                dialog.close();
                win.show_financial_result(v);
            }
            Err(e) => win.set_error_status(&e.message),
        }
    });
}

/// Character dialog (doc 06 §5.3): character ↔ code point / UTF-8 bytes /
/// UTF-16 code units; Insert places the number into the display.
pub fn open_character_dialog(
    parent: &impl gtk::glib::object::IsA<gtk::Widget>,
    win: &Rc<CalcWindow>,
) {
    let dialog = adw::Dialog::new();
    dialog.set_title("Character");
    // Floating sheet with 18 px corners; action buttons use the
    // action-button look (doc 13 §8.4).
    dialog.add_css_class("char-dialog");
    let root = gtk::Box::new(Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);

    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some("Character or code"));
    root.append(&entry);

    let combo =
        gtk::DropDown::from_strings(&["Unicode code point", "UTF-8 bytes", "UTF-16 code units"]);
    root.append(&combo);

    let preview = gtk::Label::new(None);
    preview.set_selectable(true);
    root.append(&preview);

    let insert_btn = gtk::Button::with_label("Insert");
    insert_btn.add_css_class("suggested-action");
    insert_btn.add_css_class("action-button");
    root.append(&insert_btn);

    let _dlg = dialog.clone();
    let entry2 = entry.clone();
    let combo2 = combo.clone();
    let win2 = Rc::downgrade(win);
    let dlg2 = dialog.clone();
    let preview_in = preview.clone();
    let preview_r = preview.clone();
    let recompute: Rc<dyn Fn()> = Rc::new(move || {
        let text = entry2.text().to_string();
        let mode = combo2.selected();
        let Some(ch) = text.chars().next() else {
            preview_r.set_text("");
            return;
        };
        let value: u64 = match mode {
            0 => ch as u64,
            1 => text
                .as_bytes()
                .iter()
                .fold(0u64, |acc, b| (acc << 8) | *b as u64),
            2 => {
                let mut units: Vec<u32> = text.encode_utf16().map(|u| u as u32).collect();
                // Surrogate pair encoded as a 32-bit number (doc 06 §5.3).
                if units.len() == 2 {
                    let v = ((units[0] << 16) | units[1]) as u64;
                    units.clear();
                    v
                } else {
                    units.remove(0) as u64
                }
            }
            _ => 0,
        };
        preview_r.set_text(&value.to_string());
    });
    entry.connect_changed({
        let r = Rc::clone(&recompute);
        move |_| r()
    });
    combo.connect_selected_notify({
        let r = Rc::clone(&recompute);
        move |_| r()
    });

    insert_btn.connect_clicked(move |_| {
        if let Some(win) = win2.upgrade() {
            let t = preview_in.text();
            if !t.is_empty() {
                win.display.borrow_mut().insert(t.as_ref(), false);
            }
        }
        dlg2.close();
    });

    dialog.set_child(Some(&root));
    dialog.present(Some(parent));
}

/// Keyboard shortcuts window (doc 06 §12: sections General, Switching
/// Modes, Programming Mode, Keyboard Entry, Equation).
pub fn open_shortcuts(parent: &impl gtk::glib::object::IsA<gtk::Window>) {
    let win = gtk::ShortcutsWindow::builder()
        .title("Keyboard Shortcuts")
        .build();
    let section = gtk::ShortcutsSection::builder()
        .section_name("general")
        .build();
    let group = gtk::ShortcutsGroup::builder().title("General").build();
    let entries: [(&str, &str); 16] = [
        ("Solve", "Return"),
        ("Clear display", "Escape"),
        ("Copy", "<Ctrl>C"),
        ("Paste", "<Ctrl>V"),
        ("Undo", "<Ctrl>Z"),
        ("Redo", "<Ctrl><Shift>Z"),
        ("Clear history", "<Ctrl>Escape"),
        ("New window", "<Ctrl>N"),
        ("Close window", "<Ctrl>W"),
        ("Quit", "<Ctrl>Q"),
        ("Preferences", "<Ctrl>comma"),
        ("Basic mode", "<Ctrl><Alt>B"),
        ("Advanced mode", "<Ctrl><Alt>A"),
        ("Financial mode", "<Ctrl><Alt>F"),
        ("Programming mode", "<Ctrl><Alt>P"),
        ("Keyboard mode", "<Ctrl><Alt>K"),
    ];
    for (title, accel) in entries {
        let item = gtk::ShortcutsShortcut::builder()
            .title(title)
            .accelerator(accel)
            .build();
        group.append(&item);
    }
    section.append(&group);
    win.set_child(Some(&section));
    win.set_transient_for(Some(parent));
    win.present();
}

/// About window (doc 06 §3): libadwaita's AdwAboutWindow, the GNOME 51
/// reference look — app icon, name, developer, version chip and the
/// Website / Report an Issue / Credits / Legal rows.
pub fn open_about(parent: &impl gtk::glib::object::IsA<gtk::Window>) {
    // doc 06 §3 mirrors upstream's About display; Radix itself is MIT
    // (D26), so the Legal page states Radix's own license.
    let about = adw::AboutWindow::builder()
        .transient_for(parent)
        .modal(true)
        .application_icon("io.github.hakergeniusz.Radix")
        .application_name("Calculator")
        .developer_name("hakergeniusz")
        .version(env!("CARGO_PKG_VERSION"))
        .website("https://github.com/hakergeniusz/radix")
        .issue_url("https://github.com/hakergeniusz/radix/issues")
        .copyright("© 2026 The Radix Authors")
        .license_type(gtk::License::MitX11)
        .build();
    about.add_credit_section(
        Some("Exchange Rate Data Providers"),
        &[
            "European Central Bank",
            "International Monetary Fund",
            "United Nations Treasury",
            "Bank of Canada",
        ],
    );
    about.present();
}
