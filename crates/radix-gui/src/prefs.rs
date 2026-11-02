//! Preferences dialog (doc 06 §8): an Adw.PreferencesDialog over the
//! window; every change applies immediately to the live settings.

use crate::win::CalcWindow;
use adw::prelude::*;
use gtk::glib;
use radix_core::currency::CURRENCIES;
use radix_core::settings::{AngleSetting, CurrencyDisplay};
use std::rc::Rc;

pub fn open_preferences(win: &Rc<CalcWindow>) {
    let dialog = adw::PreferencesDialog::new();
    dialog.set_title(&crate::i18n::gettext("Preferences"));
    dialog.set_search_enabled(true);

    let page = adw::PreferencesPage::new();
    page.set_title(&crate::i18n::gettext("General"));
    page.set_icon_name(Some("preferences-system-symbolic"));
    dialog.add(&page);

    let group = adw::PreferencesGroup::new();
    group.set_title(&crate::i18n::gettext("Calculation"));
    page.add(&group);

    // Number of decimals (accuracy, doc 06 §8).
    let accuracy = adw::SpinRow::with_range(0.0, 15.0, 1.0);
    accuracy.set_title(&crate::i18n::gettext("Number of decimals"));
    accuracy.set_value(win.settings.borrow().accuracy.max(0) as f64);
    group.add(&accuracy);

    // Trailing zeroes / thousands separators.
    let zeroes = adw::SwitchRow::new();
    zeroes.set_title(&crate::i18n::gettext("Trailing zeroes"));
    zeroes.set_active(win.settings.borrow().show_zeroes);
    group.add(&zeroes);

    let thousands = adw::SwitchRow::new();
    thousands.set_title(&crate::i18n::gettext("Thousands separators"));
    thousands.set_active(win.settings.borrow().show_thousands);
    group.add(&thousands);

    // Angle units.
    let angle = adw::ComboRow::new();
    angle.set_title(&crate::i18n::gettext("Angle units"));
    angle.set_model(Some(&gtk::StringList::new(&[
        &crate::i18n::gettext("Degrees"), &crate::i18n::gettext("Radians"), &crate::i18n::gettext("Gradians"),
    ])));
    angle.set_selected(match win.settings.borrow().angle_units {
        AngleSetting::Degrees => 0,
        AngleSetting::Radians => 1,
        AngleSetting::Gradians => 2,
    });
    group.add(&angle);

    // Word size.
    let word = adw::ComboRow::new();
    word.set_title(&crate::i18n::gettext("Word size"));
    word.set_model(Some(&gtk::StringList::new(&[
        "8-bit", "16-bit", "32-bit", "64-bit",
    ])));
    word.set_selected(match win.settings.borrow().word_size {
        8 => 0,
        16 => 1,
        32 => 2,
        _ => 3,
    });
    group.add(&word);

    let group2 = adw::PreferencesGroup::new();
    group2.set_title(&crate::i18n::gettext("Currency"));
    page.add(&group2);

    // Refresh interval.
    let refresh = adw::ComboRow::new();
    refresh.set_title(&crate::i18n::gettext("Currency refresh interval"));
    refresh.set_model(Some(&gtk::StringList::new(&[&crate::i18n::gettext("Never"), &crate::i18n::gettext("Daily"), &crate::i18n::gettext("Weekly")])));
    refresh.set_selected(match win.settings.borrow().refresh_interval {
        0 => 0,
        604800 => 2,
        _ => 1, // 86400
    });
    group2.add(&refresh);

    // Currency display.
    let display = adw::ComboRow::new();
    display.set_title(&crate::i18n::gettext("Currency display"));
    display.set_model(Some(&gtk::StringList::new(&[
        &crate::i18n::gettext("Currency Code"),
        &crate::i18n::gettext("Currency Name"),
        &crate::i18n::gettext("Both"),
    ])));
    display.set_selected(match win.settings.borrow().currency_display {
        CurrencyDisplay::Code => 0,
        CurrencyDisplay::Name => 1,
        CurrencyDisplay::Both => 2,
    });
    group2.add(&display);

    // Currency completion: the currency/favorite subset of
    // `enabled-completions` (doc 06 §10).
    let completion = adw::ComboRow::new();
    completion.set_title(&crate::i18n::gettext("Currency completion"));
    completion.set_model(Some(&gtk::StringList::new(&[
        &crate::i18n::gettext("All currencies"),
        &crate::i18n::gettext("Favorites only"),
        &crate::i18n::gettext("None"),
    ])));
    {
        let ec = &win.settings.borrow().enabled_completions;
        let has_cur = ec.iter().any(|s| s == "currency");
        let has_fav = ec.iter().any(|s| s == "favorite");
        completion.set_selected(if has_cur {
            0
        } else if has_fav {
            1
        } else {
            2
        });
    }
    group2.add(&completion);

    // Favorite currencies subpage (doc 06 §8): a switch per currency.
    let fav_page = adw::PreferencesPage::new();
    fav_page.set_title(&crate::i18n::gettext("Favorite Currencies"));
    fav_page.set_icon_name(Some("starred-symbolic"));
    let fav_group = adw::PreferencesGroup::new();
    fav_group.set_title(&crate::i18n::gettext("Show in conversion lists first"));
    fav_page.add(&fav_group);
    for c in CURRENCIES {
        let row = adw::SwitchRow::new();
        row.set_title(&format!("{} ({})", c.code, c.name));
        row.set_active(
            win.settings
                .borrow()
                .favorite_currencies
                .iter()
                .any(|s| s == c.code),
        );
        {
            let win = Rc::downgrade(win);
            let code = c.code.to_string();
            row.connect_active_notify(move |row| {
                let Some(win) = win.upgrade() else { return };
                let mut st = win.settings.borrow_mut();
                let fav = &mut st.favorite_currencies;
                if row.is_active() {
                    if !fav.iter().any(|s| s == &code) {
                        fav.push(code.clone());
                    }
                } else {
                    fav.retain(|s| s != &code);
                }
            });
        }
        fav_group.add(&row);
    }
    dialog.add(&fav_page);

    // ---- live bindings (doc 06 §8: all changes apply immediately) -----
    {
        let win = Rc::downgrade(win);
        accuracy.connect_changed(move |row| {
            let Some(win) = win.upgrade() else { return };
            win.settings.borrow_mut().accuracy = row.value() as i32;
            win.apply_settings_to_serializer();
        });
    }
    {
        let win = Rc::downgrade(win);
        zeroes.connect_active_notify(move |row| {
            let Some(win) = win.upgrade() else { return };
            win.settings.borrow_mut().show_zeroes = row.is_active();
            win.apply_settings_to_serializer();
        });
    }
    {
        let win = Rc::downgrade(win);
        thousands.connect_active_notify(move |row| {
            let Some(win) = win.upgrade() else { return };
            win.settings.borrow_mut().show_thousands = row.is_active();
            win.apply_settings_to_serializer();
        });
    }
    {
        let win = Rc::downgrade(win);
        angle.connect_selected_notify(move |row| {
            let Some(win) = win.upgrade() else { return };
            win.settings.borrow_mut().angle_units = match row.selected() {
                1 => AngleSetting::Radians,
                2 => AngleSetting::Gradians,
                _ => AngleSetting::Degrees,
            };
            win.apply_settings_to_serializer();
        });
    }
    {
        let win = Rc::downgrade(win);
        word.connect_selected_notify(move |row| {
            let Some(win) = win.upgrade() else { return };
            let ws = [8i32, 16, 32, 64][row.selected() as usize];
            win.settings.borrow_mut().word_size = ws;
            win.apply_settings_to_serializer();
        });
    }
    {
        let win = Rc::downgrade(win);
        refresh.connect_selected_notify(move |row| {
            let Some(win) = win.upgrade() else { return };
            // Preferences change triggers an immediate re-refresh (doc 08).
            let interval = [0i64, 86400, 604800][row.selected() as usize];
            win.settings.borrow_mut().refresh_interval = interval;
            win.restart_currency_refresh();
        });
    }
    {
        let win = Rc::downgrade(win);
        display.connect_selected_notify(move |row| {
            let Some(win) = win.upgrade() else { return };
            win.settings.borrow_mut().currency_display = match row.selected() {
                0 => CurrencyDisplay::Code,
                1 => CurrencyDisplay::Name,
                _ => CurrencyDisplay::Both,
            };
        });
    }
    {
        let win = Rc::downgrade(win);
        completion.connect_selected_notify(move |row| {
            let Some(win) = win.upgrade() else { return };
            // Currency/favorite subset; the other providers keep their
            // current state (doc 06 §10).
            let mut ec = win.settings.borrow_mut().enabled_completions.clone();
            match row.selected() {
                0 => {
                    if !ec.iter().any(|s| s == "currency") {
                        ec.push("currency".into());
                    }
                    ec.retain(|s| s != "favorite");
                }
                1 => {
                    ec.retain(|s| s != "currency");
                    if !ec.iter().any(|s| s == "favorite") {
                        ec.push("favorite".into());
                    }
                }
                _ => {
                    ec.retain(|s| s != "currency" && s != "favorite");
                }
            }
            win.settings.borrow_mut().enabled_completions = ec;
        });
    }

    dialog.present(Some(&win.window));
    let _ = glib::ControlFlow::Break; // keep glib import used on all builds
}
