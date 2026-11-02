//! Converter panel (doc 06 §6): the window-level panel shown only in
//! conversion mode — category dropdown, from/to unit dropdowns, and two
//! fully synchronized value fields with live conversion.

use crate::solver;
use crate::win::CalcWindow;
use gtk::glib;
use gtk::prelude::*;
use radix_core::currency::CURRENCIES;

use radix_core::numfuncs::pi_const;
use radix_core::serializer::{NumberFormat, Serializer};
use radix_core::settings::{CurrencyDisplay, Settings};
use radix_core::units::{self, UnitDef};
use radix_core::Number;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct ConverterPanel {
    pub root: gtk::Box,
    category_dd: gtk::DropDown,
    from_dd: gtk::DropDown,
    to_dd: gtk::DropDown,
    base_dd: gtk::DropDown,
    from_entry: gtk::Entry,
    to_entry: gtk::Entry,
    fields: gtk::Box,
    /// Status page ("Exchange Rates Not Loaded" + refresh link / spinner).
    currency_status: adw::StatusPage,
    refresh_btn: gtk::Button,
    refresh_spinner: gtk::Spinner,
    state: RefCell<State>,
    /// Current (key, display) unit pairs behind both unit dropdowns (D33):
    /// the model holds translated display labels, selection maps to keys
    /// by index.
    unit_items: RefCell<Vec<(String, String)>>,
}

struct State {
    /// Current category key.
    category: String,
    /// Per-category last-used (from, to) unit names, in-session (doc 06 §6).
    session_units: HashMap<String, (String, String)>,
    /// Reentrancy guard for field <-> recompute cycles.
    updating: bool,
    /// Number Base category radix.
    base: u32,
}

/// Sort categories by display name (doc 06 §6).
fn categories_sorted() -> Vec<(&'static str, &'static str)> {
    let mut cats: Vec<_> = radix_core::units::CATEGORY_ORDER.to_vec();
    cats.sort_by_key(|(_, disp)| disp.to_string());
    cats
}

/// Unit display name list for a category: static unit defs, or the
/// currency list (favorites first, then by code — doc 06 §6).
fn unit_list(category: &str, settings: &Settings) -> Vec<(String, String)> {
    if category == "currency" {
        let mut out: Vec<(String, String)> = CURRENCIES
            .iter()
            .map(|c| (c.code.to_string(), currency_label(c.code, c.name, settings)))
            .collect();
        let fav = &settings.favorite_currencies;
        out.sort_by_key(|(code, _)| {
            let fav_rank = if fav.contains(code) { 0 } else { 1 };
            (fav_rank, code.clone())
        });
        out
    } else {
        units::UNITS
            .iter()
            .filter(|u| u.category == category)
            // Key stays the English def name (unit_by_name matches it);
            // only the display label is translated (D33).
            .map(|u| (u.name.to_string(), crate::i18n::gettext(u.name)))
            .collect()
    }
}

fn currency_label(code: &str, name: &str, settings: &Settings) -> String {
    match settings.currency_display {
        CurrencyDisplay::Code => code.to_string(),
        CurrencyDisplay::Name => name.to_string(),
        CurrencyDisplay::Both => format!("{code} ({name})"),
    }
}

/// Unit name → def within a category.
fn unit_by_name(category: &str, name: &str) -> Option<&'static UnitDef> {
    units::UNITS
        .iter()
        .find(|u| u.category == category && u.name == name)
}

/// Persisted unit identifiers are keys ("degree", settings defaults), while
/// live selections store display names ("Degrees (Decimal)"). Accept either
/// when restoring a last-used pair (doc 06 §6).
fn resolve_unit_name(category: &str, stored: &str) -> Option<String> {
    if stored.is_empty() {
        return None;
    }
    if unit_by_name(category, stored).is_some() {
        return Some(stored.to_string());
    }
    units::UNITS
        .iter()
        .find(|u| u.category == category && u.symbols.contains(&stored))
        .map(|u| u.name.to_string())
}

/// Currency code for a converter list entry (list entries are codes for the
/// currency category; display label may be name/both — resolve back).
fn currency_code_from_label(label: &str, settings: &Settings) -> Option<String> {
    if settings.currency_display == CurrencyDisplay::Code {
        return Some(label.to_string());
    }
    for c in CURRENCIES {
        if currency_label(c.code, c.name, settings) == label {
            return Some(c.code.to_string());
        }
    }
    None
}

impl ConverterPanel {
    pub fn new(win: &Rc<CalcWindow>) -> Rc<Self> {
        // Second card surface (doc 13 §7): same 12 px margins/radius as the
        // display card (CSS, D24); transparent category dropdown; 800-weight
        // 181 % value fields. The panel absorbs the leftover height with the
        // value pairs vertically centered (upstream math-converter.blp).
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("card");
        root.add_css_class("display-container");
        root.add_css_class("conv-card");
        root.set_vexpand(true);

        // Category row.
        let cats = categories_sorted();
        let cat_names: Vec<String> = cats.iter().map(|(_, d)| crate::i18n::gettext(d)).collect();
        let cat_list =
            gtk::StringList::new(&cat_names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let category_dd = gtk::DropDown::new(Some(cat_list), gtk::Expression::NONE);
        category_dd.set_halign(gtk::Align::Start);
        category_dd.set_hexpand(true);

        // From / To unit dropdowns (searchable when > 10 units, doc 06 §6).
        // halign Start keeps each button at its natural width so the chevron
        // sits right after the label (reference; D24).
        let from_dd = gtk::DropDown::new(Some(gtk::StringList::new(&[""])), gtk::Expression::NONE);
        let to_dd = gtk::DropDown::new(Some(gtk::StringList::new(&[""])), gtk::Expression::NONE);
        from_dd.set_halign(gtk::Align::Start);
        to_dd.set_halign(gtk::Align::Start);
        from_dd.set_hexpand(true);
        to_dd.set_hexpand(true);

        // Number-base radix selector (Number Base category only); the
        // selected index maps to [2, 8, 10, 16], so labels may translate.
        let base_names: Vec<String> = ["Binary", "Octal", "Decimal", "Hexadecimal"]
            .iter()
            .map(|s| crate::i18n::gettext(s))
            .collect();
        let base_list =
            gtk::StringList::new(&base_names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let base_dd = gtk::DropDown::new(Some(base_list), gtk::Expression::NONE);
        base_dd.set_visible(false);
        base_dd.set_margin_top(6);

        root.append(&category_dd);
        root.append(&base_dd);

        // Value fields, stacked: the second value sits ≈36 px below the
        // first with each unit dropdown below its value (doc 13 §7); the
        // block is centered in the card's leftover height (D24).
        let from_entry = gtk::Entry::new();
        let to_entry = gtk::Entry::new();
        for e in [&from_entry, &to_entry] {
            e.set_direction(gtk::TextDirection::Ltr);
            e.set_width_chars(12);
            e.add_css_class("conv-value");
            e.set_has_frame(false);
        }
        to_entry.set_margin_top(36);
        let fields = gtk::Box::new(gtk::Orientation::Vertical, 0);
        fields.set_valign(gtk::Align::Center);
        fields.set_vexpand(true);
        fields.append(&from_entry);
        fields.append(&from_dd);
        fields.append(&to_entry);
        fields.append(&to_dd);
        root.append(&fields);

        // Currency status page (doc 13 §7): unavailable icon, title, inline
        // refresh link; a full-size spinner while loading.
        let refresh_btn = gtk::Button::with_label("Refresh rates");
        refresh_btn.add_css_class("link");
        refresh_btn.add_css_class("flat");
        let refresh_spinner = gtk::Spinner::new();
        let currency_status = adw::StatusPage::new();
        currency_status.set_icon_name(Some("action-unavailable-symbolic"));
        currency_status.set_title("Exchange Rates Not Loaded");
        currency_status.set_child(Some(&refresh_btn));
        currency_status.set_margin_start(12);
        currency_status.set_margin_end(12);
        currency_status.set_vexpand(true);
        currency_status.set_visible(false);
        root.append(&currency_status);

        let state = RefCell::new(State {
            category: "angle".to_string(),
            session_units: HashMap::new(),
            updating: false,
            base: 10,
        });

        let panel = Rc::new(ConverterPanel {
            root,
            category_dd,
            from_dd,
            to_dd,
            base_dd,
            from_entry,
            to_entry,
            fields,
            currency_status,
            refresh_btn: refresh_btn.clone(),
            refresh_spinner,
            state,
            unit_items: RefCell::new(Vec::new()),
        });

        // ---- signals ----------------------------------------------------
        panel.category_dd.connect_selected_notify({
            let p = Rc::downgrade(&panel);
            let w = Rc::downgrade(win);
            move |_| {
                let (Some(p), Some(w)) = (p.upgrade(), w.upgrade()) else {
                    return;
                };
                p.on_category_changed(&w);
            }
        });
        for dd in [&panel.from_dd, &panel.to_dd] {
            let p = Rc::downgrade(&panel);
            let w = Rc::downgrade(win);
            dd.connect_selected_notify(move |_| {
                let (Some(p), Some(w)) = (p.upgrade(), w.upgrade()) else {
                    return;
                };
                p.remember_pair(&w);
                p.recompute(&w);
            });
        }
        panel.from_entry.connect_changed({
            let p = Rc::downgrade(&panel);
            let w = Rc::downgrade(win);
            move |_| {
                let (Some(p), Some(w)) = (p.upgrade(), w.upgrade()) else {
                    return;
                };
                p.field_edited(&w, true);
            }
        });
        panel.to_entry.connect_changed({
            let p = Rc::downgrade(&panel);
            let w = Rc::downgrade(win);
            move |_| {
                let (Some(p), Some(w)) = (p.upgrade(), w.upgrade()) else {
                    return;
                };
                p.field_edited(&w, false);
            }
        });
        panel.base_dd.connect_selected_notify({
            let p = Rc::downgrade(&panel);
            let w = Rc::downgrade(win);
            move |_| {
                let (Some(p), Some(w)) = (p.upgrade(), w.upgrade()) else {
                    return;
                };
                let base = [2u32, 8, 10, 16][p.base_dd.selected() as usize];
                p.state.borrow_mut().base = base;
                p.recompute(&w);
            }
        });
        refresh_btn.connect_clicked({
            let p = Rc::downgrade(&panel);
            let w = Rc::downgrade(win);
            move |_| {
                let (Some(p), Some(w)) = (p.upgrade(), w.upgrade()) else {
                    return;
                };
                p.refresh_rates(&w);
            }
        });

        panel
    }

    // -----------------------------------------------------------------
    // Selection plumbing
    // -----------------------------------------------------------------

    fn set_dropdown_items(dd: &gtk::DropDown, names: &[String]) {
        let list = gtk::StringList::new(&names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        dd.set_model(Some(&list));
        dd.set_enable_search(names.len() > 10);
    }

    /// The unit key behind a dropdown's current selection: the model holds
    /// translated display labels, so the key comes from the pair list by
    /// index (D33).
    fn selected_key(&self, dd: &gtk::DropDown) -> Option<String> {
        let idx = dd.selected() as usize;
        self.unit_items.borrow().get(idx).map(|(k, _)| k.clone())
    }

    /// Select a unit by its (untranslated) key.
    fn select_key(&self, dd: &gtk::DropDown, key: &str) {
        let items = self.unit_items.borrow();
        if let Some(idx) = items.iter().position(|(k, _)| k == key) {
            dd.set_selected(idx as u32);
        }
    }

    fn on_category_changed(self: &Rc<Self>, win: &Rc<CalcWindow>) {
        let idx = self.category_dd.selected() as usize;
        let cats = categories_sorted();
        let key = cats.get(idx).map(|(k, _)| *k).unwrap_or("angle");
        self.state.borrow_mut().category = key.to_string();
        let is_base = key == "base";
        self.base_dd.set_visible(is_base);
        let base = self.state.borrow().base;
        self.base_dd.set_selected(match base {
            2 => 0,
            8 => 1,
            16 => 3,
            _ => 2,
        });
        self.update_hex_buttons(win);

        // Rebuilding the models re-fires selected_notify; hold the updating
        // guard so remember_pair/recompute skip the intermediate states.
        self.state.borrow_mut().updating = true;
        let settings = win.settings.borrow();
        let items = unit_list(key, &settings);
        let keys: Vec<String> = items.iter().map(|(k, _)| k.clone()).collect();
        let names: Vec<String> = items.iter().map(|(_, d)| d.clone()).collect();
        drop(settings);
        *self.unit_items.borrow_mut() = items;
        Self::set_dropdown_items(&self.from_dd, &names);
        Self::set_dropdown_items(&self.to_dd, &names);

        // Last-used pair: session map first, then persisted settings for the
        // startup category, else defaults (first two units).
        let (from, to) = {
            let st = self.state.borrow();
            st.session_units
                .get(key)
                .cloned()
                .or_else(|| {
                    let settings = win.settings.borrow();
                    if settings.unit_category == key {
                        let f = settings
                            .source_units
                            .first()
                            .and_then(|s| resolve_unit_name(key, s));
                        let t = settings
                            .target_units
                            .first()
                            .and_then(|s| resolve_unit_name(key, s));
                        if let (Some(f), Some(t)) = (f, t) {
                            return Some((f, t));
                        }
                    }
                    None
                })
                .unwrap_or_else(|| {
                    let f = keys.first().cloned().unwrap_or_default();
                    let t = keys.get(1).cloned().unwrap_or(f.clone());
                    (f, t)
                })
        };
        self.select_key(&self.from_dd, &from);
        self.select_key(&self.to_dd, &to);
        self.state.borrow_mut().updating = false;

        // Currency category: check rate availability.
        self.fields.set_visible(key != "currency");
        self.currency_status
            .set_visible(key == "currency" && !self.rates_loaded(win));
        self.recompute(win);
    }

    fn update_hex_buttons(self: &Rc<Self>, win: &Rc<CalcWindow>) {
        let (is_base, base, category) = {
            let st = self.state.borrow();
            (st.category == "base", st.base, st.category.clone())
        };
        let pad = win.conv_pad.borrow();
        if let Some(pad) = pad.as_ref() {
            // Decimal and hex digits swap inside one grid cell (a hidden
            // grid row would still count toward the keypad minimum).
            pad.digits_grid.set_visible(!is_base);
            for b in &pad.hex_buttons {
                b.set_visible(is_base);
                b.set_sensitive(is_base && base == 16);
            }
            // Contextual last-cell button per category (doc 06 §5.5):
            // °′″ for angle, rate details for currency, π elsewhere; the
            // Number Base category shows the hex block instead.
            pad.dms_btn.set_visible(category == "angle");
            pad.rate_btn.set_visible(category == "currency");
            pad.pi_btn
                .set_visible(category != "angle" && category != "currency" && !is_base);
        }
    }

    fn rates_loaded(self: &Rc<Self>, win: &Rc<CalcWindow>) -> bool {
        let table = win.currencies.lock().unwrap();
        table.rates.contains_key("USD") || !table.rates.is_empty()
    }

    pub fn refresh_rates(self: &Rc<Self>, win: &Rc<CalcWindow>) {
        // Full-size spinner while loading (doc 13 §7).
        self.currency_status.set_child(Some(&self.refresh_spinner));
        self.refresh_spinner.start();
        let rx = solver::spawn_rate_refresh_forced(&win.currencies);
        let p = Rc::downgrade(self);
        let w = Rc::downgrade(win);
        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            if rx.try_recv().is_ok() {
                if let (Some(p), Some(w)) = (p.upgrade(), w.upgrade()) {
                    p.refresh_spinner.stop();
                    p.currency_status.set_child(Some(&p.refresh_btn));
                    p.recompute(&w);
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    /// Current from/to unit identity: static def name or currency code.
    #[allow(dead_code)]
    fn current_units(self: &Rc<Self>) -> (Option<String>, Option<String>) {
        (
            self.selected_key(&self.from_dd),
            self.selected_key(&self.to_dd),
        )
    }

    // -----------------------------------------------------------------
    // Conversion computation
    // -----------------------------------------------------------------

    fn recompute(self: &Rc<Self>, win: &Rc<CalcWindow>) {
        if self.state.borrow().updating {
            return;
        }
        self.state.borrow_mut().updating = true;
        let cat = self.state.borrow().category.clone();

        let from = self.selected_key(&self.from_dd);
        let to = self.selected_key(&self.to_dd);
        let focused_from = self.from_entry.has_focus();
        let focused_to = self.to_entry.has_focus();

        // Decide the source of truth: the focused field, else from-field.
        let (src_text, from_side) = if focused_to && !focused_from {
            (self.to_entry.text().to_string(), false)
        } else {
            (self.from_entry.text().to_string(), true)
        };

        match self.convert_text(win, &cat, &from, &to, &src_text) {
            Ok(result) => {
                if from_side {
                    self.to_entry
                        .set_text(&self.format_value(win, &result, focused_to));
                } else {
                    self.from_entry
                        .set_text(&self.format_value(win, &result, focused_from));
                }
                self.currency_status.set_visible(false);
                if cat == "currency" {
                    self.fields.set_visible(true);
                }
            }
            Err(e) => {
                // Invalid input shows "Error" in the other field; 0 is used
                // for computation (doc 06 §6).
                let other = if from_side {
                    &self.to_entry
                } else {
                    &self.from_entry
                };
                if src_text.trim().is_empty() {
                    other.set_text("");
                } else {
                    other.set_text("Error");
                }
                let _ = e;
            }
        }
        self.state.borrow_mut().updating = false;
    }

    fn field_edited(self: &Rc<Self>, win: &Rc<CalcWindow>, _from: bool) {
        self.recompute(win);
    }

    /// Convert `text` (value in the source unit) to the target unit.
    #[allow(clippy::too_many_arguments)]
    fn convert_text(
        self: &Rc<Self>,
        win: &Rc<CalcWindow>,
        category: &str,
        from: &Option<String>,
        to: &Option<String>,
        text: &str,
    ) -> Result<Number, String> {
        let (Some(from), Some(to)) = (from, to) else {
            return Err("no unit".into());
        };
        let value = self.parse_field(win, text)?;
        if category == "currency" {
            let table = win.currencies.lock().unwrap();
            let code_from = {
                let settings = win.settings.borrow();
                currency_code_from_label(from, &settings)
                    .or_else(|| Some(from.clone()))
                    .unwrap_or_default()
            };
            let code_to = {
                let settings = win.settings.borrow();
                currency_code_from_label(to, &settings).unwrap_or_else(|| to.clone())
            };
            let prec = win.settings.borrow().precision.max(0) as u32;
            return table
                .convert(&value, &code_from, &code_to, prec)
                .map_err(|e| e.message);
        }
        let f = unit_by_name(category, from).ok_or("unit")?;
        let t = unit_by_name(category, to).ok_or("unit")?;
        let prec = win.settings.borrow().precision.max(0) as u32;
        units::convert(&value, f, t, prec).map_err(|e| e.message)
    }

    /// Field text → Number: decimal (in the current base for the Number
    /// Base category), thousands separators stripped, π forms supported
    /// (`π`, `−π`, `kπ` — doc 06 §6).
    fn parse_field(self: &Rc<Self>, win: &Rc<CalcWindow>, text: &str) -> Result<Number, String> {
        let prec = win.settings.borrow().precision.max(0) as u32;
        let base = self.state.borrow().base;
        let mut t = text.replace(['\u{202F}', ','], "").trim().to_string();
        if t.is_empty() {
            return Ok(Number::int(0, prec));
        }
        let neg = t.starts_with('−') || t.starts_with('-');
        if neg {
            t = t[1..].to_string();
        }
        let mag = self.parse_magnitude(&t, base, prec)?;
        Ok(if neg { mag.neg() } else { mag })
    }

    fn parse_magnitude(self: &Rc<Self>, t: &str, base: u32, prec: u32) -> Result<Number, String> {
        // π forms: `π`, `kπ` (doc 06 §6).
        if let Some(k) = t.strip_suffix('π') {
            let k = k.trim_end_matches('·').trim();
            let mult = if k.is_empty() {
                Number::int(1, prec)
            } else {
                radix_core::number::parse_decimal_rational(k)
                    .map(|r| Number::rational(r, prec))
                    .ok_or("bad π multiplier")?
            };
            let pi = Number::float(pi_const(prec));
            return Ok(mult.mul(&pi));
        }
        if base == 10 {
            return radix_core::number::parse_decimal_rational(t)
                .map(|r| Number::rational(r, prec))
                .ok_or_else(|| "invalid number".to_string());
        }
        // Non-decimal base: integer part exact, fraction via f64.
        let (int_part, frac_part) = match t.split_once('.') {
            Some((a, b)) => (a, Some(b)),
            None => (t, None),
        };
        let int_val = i128::from_str_radix(int_part, base).map_err(|_| "invalid number")?;
        let mut v = Number::int(int_val as i64, prec);
        if let Some(frac) = frac_part {
            let mut f = 0f64;
            for (i, c) in frac.chars().enumerate() {
                let d = c.to_digit(16).ok_or("invalid number")? as f64;
                if d as u32 >= base {
                    return Err("invalid number".into());
                }
                f += d / (base as f64).powi(i as i32 + 1);
            }
            v = v.add(&Number::from_f64(f, prec));
        }
        Ok(v)
    }

    /// Render a value for a field: focused → plain fixed format (no
    /// grouping); unfocused → user-formatted; Number Base category renders
    /// in the selected base; DMS unit formats d°m′s″ (doc 06 §6).
    fn format_value(self: &Rc<Self>, win: &Rc<CalcWindow>, v: &Number, focused: bool) -> String {
        let s = win.settings.borrow();
        let cat = self.state.borrow().category.clone();
        let base = self.state.borrow().base;
        let ser = Serializer {
            accuracy: s.accuracy.max(0) as u32,
            show_thousands: if focused { false } else { s.show_thousands },
            show_zeroes: if focused { false } else { s.show_zeroes },
            format: NumberFormat::Fixed,
            radix: '.',
            thousands: ',',
            repr_base: if cat == "base" { base } else { 10 },
            parser_base: if cat == "base" { base } else { 10 },
            ..Serializer::default()
        };
        // DMS formatting when either endpoint is the DMS angle unit.
        let dms = [
            self.selected_key(&self.from_dd),
            self.selected_key(&self.to_dd),
        ]
        .iter()
        .any(|n| n.as_deref() == Some("Degrees (D°M′S″)"));
        if cat == "angle" && dms {
            return ser.serialize_dms(v);
        }
        ser.serialize(v).unwrap_or_else(|_| "Error".into())
    }

    // -----------------------------------------------------------------
    // Keypad routing (doc 06 §5.5: conversions are live, no equals)
    // -----------------------------------------------------------------

    fn focused_entry(self: &Rc<Self>) -> gtk::Entry {
        if self.to_entry.has_focus() && !self.from_entry.has_focus() {
            self.to_entry.clone()
        } else {
            self.from_entry.clone()
        }
    }

    pub fn insert_text(self: &Rc<Self>, text: &str) {
        let e = self.focused_entry();
        let mut pos = e.position();
        if pos < 0 {
            pos = e.text().len() as i32;
        }
        e.insert_text(text, &mut pos);
    }

    pub fn backspace(self: &Rc<Self>) {
        let e = self.focused_entry();
        let mut pos = e.position();
        if pos < 0 {
            pos = e.text().len() as i32;
        }
        if pos > 0 {
            e.delete_text(pos - 1, pos);
            e.set_position(pos - 1);
        }
    }

    pub fn clear(self: &Rc<Self>) {
        self.from_entry.set_text("");
        self.to_entry.set_text("");
    }

    pub fn insert_dms(self: &Rc<Self>) {
        self.insert_text("°");
    }

    pub fn swap(self: &Rc<Self>, win: &Rc<CalcWindow>) {
        let f = self.selected_key(&self.from_dd);
        let t = self.selected_key(&self.to_dd);
        if let (Some(f), Some(t)) = (f, t) {
            self.select_key(&self.from_dd, &t);
            self.select_key(&self.to_dd, &f);
            self.remember_pair(win);
            self.recompute(win);
        }
    }

    fn remember_pair(self: &Rc<Self>, _win: &Rc<CalcWindow>) {
        // Skip programmatic model rebuilds — only user-made selections are
        // "last used" (doc 06 §6).
        if self.state.borrow().updating {
            return;
        }
        let cat = self.state.borrow().category.clone();
        if let (Some(f), Some(t)) = (
            self.selected_key(&self.from_dd),
            self.selected_key(&self.to_dd),
        ) {
            self.state.borrow_mut().session_units.insert(cat, (f, t));
        }
    }

    /// Persisted pair for the current category (doc 06 §6: only the current
    /// category's pair is written at shutdown).
    pub fn snapshot_settings(self: &Rc<Self>, settings: &mut Settings) {
        let cat = self.state.borrow().category.clone();
        settings.unit_category = cat.clone();
        if let Some((f, t)) = self.state.borrow().session_units.get(&cat).cloned() {
            settings.source_units = vec![f];
            settings.target_units = vec![t];
        }
    }

    /// Apply persisted category/units at startup (doc 06 §6).
    pub fn apply_startup(self: &Rc<Self>, win: &Rc<CalcWindow>) {
        let settings = win.settings.borrow();
        let idx = categories_sorted()
            .iter()
            .position(|(k, _)| *k == settings.unit_category)
            .unwrap_or(0) as u32;
        drop(settings);
        // set_selected only notifies on change; at startup the dropdown is
        // already at 0, so drive the refresh explicitly in that case.
        let prev = self.category_dd.selected();
        self.category_dd.set_selected(idx);
        if prev == idx {
            self.on_category_changed(win);
        }
        // Seed "0" so the panel reads as a live 0→0 pair instead of a blank
        // from-field over the computed to-value (doc 06 §6 live sync).
        if self.from_entry.text().is_empty() {
            self.from_entry.set_text("0");
        }
    }

    /// Switch category programmatically (financial's Currency Conversion
    /// button preselects Currency — doc 06 §5.4).
    pub fn select_category(self: &Rc<Self>, win: &Rc<CalcWindow>, key: &str) {
        let idx = categories_sorted()
            .iter()
            .position(|(k, _)| *k == key)
            .unwrap_or(0) as u32;
        let prev = self.category_dd.selected();
        self.category_dd.set_selected(idx);
        if prev == idx {
            self.on_category_changed(win);
        }
    }

    /// Rate details line for the status row (conversion keypad `Rate`).
    pub fn rate_details(self: &Rc<Self>, win: &Rc<CalcWindow>) -> String {
        let cat = self.state.borrow().category.clone();
        if cat != "currency" {
            return String::new();
        }
        let table = win.currencies.lock().unwrap();
        let mut lines = Vec::new();
        let codes: Vec<String> = {
            let settings = win.settings.borrow();
            [
                self.selected_key(&self.from_dd),
                self.selected_key(&self.to_dd),
            ]
            .into_iter()
            .flatten()
            .map(|n| currency_code_from_label(&n, &settings).unwrap_or(n))
            .collect()
        };
        for code in codes {
            if let Some(r) = table.rates.get(&code) {
                lines.push(format!(
                    "1 {} = {} ({}, {})",
                    code, r.value, r.provider, r.date
                ));
            }
        }
        lines.join("  ·  ")
    }
}
