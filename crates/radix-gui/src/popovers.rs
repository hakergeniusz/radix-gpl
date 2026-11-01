//! Variables and functions popovers (doc 06 §7; popover chrome per doc 13
//! §8.1: `math-popover` with zero content padding, 9 px rounded rows).
//! Content builders are shared by the basic-mode x / f(x) MenuButtons
//! (persistent popovers, refreshed on open) and the advanced-mode inverse
//! pairs (fresh popover per open).

use crate::win::CalcWindow;
use gtk::prelude::*;

use gtk::{Orientation, Popover, ScrolledWindow};
use std::rc::Rc;

/// Fill `root` with the variables view: store-name row, user variables,
/// `_`, `rand` (doc 06 §7).
fn variables_content(root: &gtk::Box, pop: &Popover, win: &Rc<CalcWindow>) {
    // Name entry + store button (doc 06 §7).
    let name_row = gtk::Box::new(Orientation::Horizontal, 6);
    let name_entry = gtk::Entry::new();
    name_entry.set_placeholder_text(Some("Variable name"));
    name_entry.set_hexpand(true);
    let store_btn = gtk::Button::from_icon_name("document-save-symbolic");
    store_btn.add_css_class("flat");
    store_btn.set_tooltip_text(Some("Store value"));
    name_row.append(&name_entry);
    name_row.append(&store_btn);
    root.append(&name_row);

    let status = gtk::Label::new(None);
    status.set_xalign(0.0);
    root.append(&status);

    let list = gtk::ListBox::new();
    let scroll = ScrolledWindow::new();
    scroll.set_child(Some(&list));
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_vexpand(true);
    root.append(&scroll);

    // Variable rows.
    let state = win.eval_state.lock().unwrap();
    let mut names: Vec<&String> = state.variables.keys().collect();
    names.sort();
    for name in names {
        let value = &state.variables[name];
        let text = format!(
            "{} = {}",
            name,
            radix_core::serializer::Serializer::register_string(value)
        );
        list.append(&popover_row(&text, name, pop, win));
    }
    drop(state);
    list.append(&popover_row("_ (previous answer)", "_", pop, win));
    list.append(&popover_row("rand", "rand", pop, win));

    store_btn.connect_clicked({
        let entry = name_entry.clone();
        let status = status.clone();
        let win = Rc::downgrade(win);
        let pop = pop.clone();
        move |_| {
            let Some(win) = win.upgrade() else { return };
            let raw = entry.text().to_string();
            // Spaces become underscores; validation per doc 04 §1.7.
            let name = raw.replace(' ', "_");
            let valid = !name.is_empty()
                && name.chars().all(|c| c.is_alphabetic())
                && !radix_core::constants::is_constant(&name)
                && !win.eval_state.lock().unwrap().functions.contains_key(&name);
            if !valid {
                status.set_text("Invalid variable name");
                entry.add_css_class("error");
                return;
            }
            status.set_text("");
            entry.remove_css_class("error");
            let value = win.display.borrow().answer_value().cloned().or_else(|| {
                // "stores the currently displayed value" — solve the
                // display text when no result is showing (doc 06 §7).
                let config = win.eval_config();
                let units = radix_core::units::UnitCatalog::new();
                let currencies = win.currencies.lock().unwrap().clone();
                let mut state = win.eval_state.lock().unwrap().clone();
                let text = win.display.borrow().canonicalize();
                if text.trim().is_empty() {
                    return None;
                }
                let out =
                    radix_core::solve::solve(&text, &config, &mut state, &units, &currencies, None);
                out.value
            });
            if let Some(v) = value {
                win.eval_state
                    .lock()
                    .unwrap()
                    .variables
                    .insert(name.clone(), v);
                let _ = radix_core::persistence::save_registers(
                    &radix_core::persistence::registers_path(),
                    &win.eval_state.lock().unwrap(),
                );
                pop.popdown();
            } else {
                status.set_text("No value to store");
            }
        }
    });
}

fn section_header(text: &str) -> gtk::Label {
    // Bold section header, separated from the list (doc 13 §8.1).
    let l = gtk::Label::new(Some(text));
    l.add_css_class("title-header");
    l.set_halign(gtk::Align::Start);
    l.set_xalign(0.0);
    l
}

/// Fill `root` with the functions view: built-ins by category (doc 06 §7).
fn functions_content(root: &gtk::Box, pop: &Popover, win: &Rc<CalcWindow>) {
    let scroll = ScrolledWindow::new();
    let list = gtk::ListBox::new();
    scroll.set_child(Some(&list));
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_vexpand(true);
    root.append(&scroll);

    for f in radix_core::functions::builtin_functions() {
        let label = format!("{}()  —  {}", f.name, f.description);
        let row = gtk::Box::new(Orientation::Horizontal, 6);
        let lbl = gtk::Label::new(Some(&label));
        lbl.set_halign(gtk::Align::Start);
        lbl.set_hexpand(true);
        row.append(&lbl);
        let pop2 = pop.clone();
        let win2 = Rc::downgrade(win);
        let name = f.name.to_string();
        let btn = gtk::Button::from_icon_name("list-add-symbolic");
        btn.add_css_class("flat");
        btn.connect_clicked(move |_| {
            pop2.popdown();
            if let Some(win) = win2.upgrade() {
                win.display.borrow_mut().insert(&format!("{name}("), false);
                win.clear_status();
            }
        });
        row.append(&btn);
        let container = gtk::ListBoxRow::new();
        container.set_child(Some(&row));
        list.append(&container);
    }
}

fn popover_row(text: &str, insert: &str, pop: &Popover, win: &Rc<CalcWindow>) -> gtk::ListBoxRow {
    let row = gtk::Box::new(Orientation::Horizontal, 6);
    let lbl = gtk::Label::new(Some(text));
    lbl.set_halign(gtk::Align::Start);
    lbl.set_hexpand(true);
    row.append(&lbl);
    let insert = insert.to_string();
    let pop = pop.clone();
    let win = Rc::downgrade(win);
    let btn = gtk::Button::from_icon_name("list-add-symbolic");
    btn.add_css_class("flat");
    btn.connect_clicked(move |_| {
        // Insert the name at the display cursor (doc 06 §7); the display
        // applies spacing and completion rules.
        pop.popdown();
        if let Some(win) = win.upgrade() {
            win.display
                .borrow_mut()
                .insert(&format!("{insert} "), false);
            win.clear_status();
        }
    });
    row.append(&btn);
    let container = gtk::ListBoxRow::new();
    container.set_child(Some(&row));
    container
}

/// Refresh the content of an x / f(x) MenuButton popover on open
/// (doc 13 §5.2: persistent popovers attached to the MenuButton).
pub fn fill_math_popover(
    root: &gtk::Box,
    pop: &Popover,
    win: &Rc<CalcWindow>,
    kind: crate::keypad::MathPopoverKind,
) {
    while let Some(c) = root.first_child() {
        root.remove(&c);
    }
    match kind {
        crate::keypad::MathPopoverKind::Variables => {
            root.append(&section_header("Variables"));
            variables_content(root, pop, win);
        }
        crate::keypad::MathPopoverKind::Functions => {
            root.append(&section_header("Functions"));
            functions_content(root, pop, win);
        }
    }
}
