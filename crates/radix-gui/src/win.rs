//! Application window (doc 06 §2–5, §9, §14): header bar, history view,
//! display with status line, keypad stack per mode, solve pipeline,
//! undo/redo, keyboard entry, and shutdown state saving.

use crate::converter::ConverterPanel;
use crate::display::{Display, InsertMode};
use crate::keypad::{self, PadAction, PadSink};
use crate::solver;
use adw::prelude::*;
use gtk::gdk::ModifierType;
use gtk::gio;
use gtk::glib::{self};
use gtk::pango;
use radix_core::currency::CurrencyTable;
use radix_core::eval::{EvalConfig, EvalState};
use radix_core::numfuncs::AngleUnit;
use radix_core::serializer::{NumberFormat, Serializer};
use radix_core::settings::{
    ButtonMode, HistoryEntry, KeyfileStore, NumberFormatSetting, Settings, SettingsStore,
};
use radix_core::Number;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct CalcWindow {
    pub window: adw::ApplicationWindow,
    pub display: Rc<RefCell<Display>>,
    pub status: gtk::Label,
    pub spinner: gtk::Spinner,
    history_box: gtk::ListBox,
    history_scroller: gtk::ScrolledWindow,
    keypad_stack: gtk::Stack,
    /// Centered mode menu button (label = active mode, doc 13 §3.2).
    mode_btn: gtk::MenuButton,
    /// Hamburger primary-menu button.
    menu_btn: gtk::MenuButton,
    undo_btn: gtk::Button,
    redo_btn: gtk::Button,
    back_btn: gtk::Button,
    backspace_btn: gtk::Button,
    /// The display card: history + entry + status (doc 13 §4).
    card: gtk::Box,
    /// The clamp's content column — carries the below-700 `small` class.
    content: gtk::Box,
    /// The expression text view (focused after solves so the caret shows).
    tv: gtk::TextView,
    /// `win.mode` stateful action (drives the mode menu radios + accels).
    mode_action: gio::SimpleAction,
    /// `win.number-format` stateful action (Result Format radios).
    fmt_action: gio::SimpleAction,
    /// `win.clear` action (disabled in conversion mode, like upstream).
    clear_action: gio::SimpleAction,
    pub settings: RefCell<Settings>,
    pub eval_state: Arc<Mutex<EvalState>>,
    pub currencies: Arc<Mutex<CurrencyTable>>,
    solve_rx: RefCell<Option<std::sync::mpsc::Receiver<solver::SolveReply>>>,
    solving: RefCell<bool>,
    /// When the running solve started (spinner only after 100 ms, doc 05 §1).
    solve_started: RefCell<Option<std::time::Instant>>,
    history: RefCell<Vec<HistoryEntry>>,
    /// Repeat-last-op bookkeeping (doc 05 §1.5).
    last_equation: RefCell<Option<String>>,
    /// Current display base (programming mode).
    base: RefCell<u32>,
    wordlen: RefCell<u32>,
    mode: RefCell<ButtonMode>,
    inverse: RefCell<bool>,
    /// Superscript/subscript mode label state (button highlighting).
    mode_indicator: gtk::Label,
    /// Programming bit panel handle.
    prog: RefCell<Option<keypad::ProgrammingPads>>,
    /// Wide-mode responsive pads (advanced/financial, doc 13 §6.1).
    advanced_pads: RefCell<Option<keypad::ResponsivePads>>,
    financial_pads: RefCell<Option<keypad::ResponsivePads>>,
    /// Conversion keypad handle (hex buttons enable per base).
    pub conv_pad: RefCell<Option<keypad::ConversionPad>>,
    /// Modes whose keypad page has been built (D32 lazy keypads).
    built_pads: RefCell<Vec<ButtonMode>>,
    /// Converter panel (conversion mode only).
    pub converter: RefCell<Option<Rc<ConverterPanel>>>,
    /// Alt+Left/Right history navigation position.
    history_nav: RefCell<Option<usize>>,
    /// Below-700 px responsive collapse (doc 13 §6.1).
    collapsed: std::cell::Cell<bool>,
    /// Undo/redo sensitivity refresh scheduled on idle.
    undo_sched: std::cell::Cell<bool>,
}

const MODE_NAMES: [(ButtonMode, &str); 6] = [
    (ButtonMode::Basic, "Basic"),
    (ButtonMode::Advanced, "Advanced"),
    (ButtonMode::Financial, "Financial"),
    (ButtonMode::Programming, "Programming"),
    (ButtonMode::Keyboard, "Keyboard"),
    (ButtonMode::Conversion, "Conversion"),
];

impl CalcWindow {
    /// Application stylesheet (doc 13): all colors are theme tokens so
    /// light/dark/high-contrast carry over; the formulas here are the
    /// app-specific layer on top of the platform look.
    fn load_css() {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(
            /* Flat header (reference parity, D25): the top bar is the same
               color as the window — libadwaita's default headerbar shade
               made the header a visibly lighter band than the content. */
            "@define-color headerbar_bg_color @window_bg_color; \
             .display-container { margin: 12px 12px 0 12px; border-radius: 12px; transition: margin 200ms cubic-bezier(0.25, 0.46, 0.45, 0.94); }
             .display-container.prog { border-radius: 12px 12px 0 0; }
             /* Below the 700 px clamp threshold the card hugs the window
                edges with no radius (upstream clamp > .small rules); the
                bit panel follows so the programming card stays seamless. */
             .small .display-container { margin-top: 0; margin-left: 0; margin-right: 0; border-radius: 0; }
             .small .bit-panel { border-radius: 0; }
             .kb-mode .display-container { margin-bottom: 12px; }
             /* History area (§4.2): lighter overlay in dark, darker in light;*/
             /* 1 px bottom border separates it from the entry. The ListBox
                paints its own view background over the card — kill it so the
                rows sit on the card tint like the reference (D24).*/
             .history-view { background: alpha(currentColor, 0.05); border-bottom: 1px solid @card_shade_color; }
             .high-contrast .history-view { border-bottom-color: @borders; }
             .history-view list { background: none; color: inherit; }
             .history-entry { border-top: 1px solid @card_shade_color; padding: 6px 3px; font-size: 1.3em; font-feature-settings: \"tnum\"; }
             .high-contrast .history-entry { border-top-color: @borders; }
             .history-entry:first-child { margin-top: -1px; }
             .history-entry .dim-eq { color: alpha(currentColor, 0.55); padding: 0 12px; }
             /* Expression entry (§4.3): transparent over the card, UI font.*/
             .display-view { background: none; font-size: 1.4em; font-feature-settings: \"tnum\"; }
             .display-view > text { background: none; }
             .status-view { min-height: 18px; margin: 0 12px 4px 12px; }
             /* Keypad (§5): the grid is a fixed 336×236 block (5×64/44 cells
                + 4 px spacing) that fills its viewport; per-class overrides.*/
             .math-buttons { min-width: 336px; min-height: 236px; }
             /* Buttons fill their homogeneous cells (the grid's 336×236
                CSS floor sets the 64×44 cell size); per-button minimums
                would force the programming hex columns (5 narrow digits
                inside 3 main columns) to full button width and overflow
                the 700 px window (D25).*/
             .math-buttons button { font-size: 1.1em; padding: 0; }
             .math-buttons button.number-button { font-size: 1.2em; font-weight: bolder; background: alpha(currentColor, 0.2); }
             .math-buttons button.number-button:hover { background: alpha(currentColor, 0.25); }
             .math-buttons button.number-button:active { background: alpha(currentColor, 0.35); }
             .math-buttons button.operator-button { font-size: 1.6em; }
             .math-buttons button.suggested-action { font-size: 1.6em; }
             .math-buttons button.clear-button { font-size: 1.2em; font-weight: bolder; }
             /* ⇧⁻¹ inverse-modifier toggle (doc 13 §5.2): accent-tinted and
                accent-texted while off, solid accent while on (reference).*/
             .math-buttons button.accent { background: alpha(@accent_color, 0.15); color: @accent_color; }
             .math-buttons button.accent:checked { background-color: @accent_bg_color; color: @accent_fg_color; }
             /* Math popovers (§8.1).*/
             .math-popover row { border-radius: 9px; min-height: 28px; }
             .math-popover .title-header { font-weight: bold; margin: 10px 18px 6px 18px; }
             .math-popover scrollbar slider { min-height: 26px; }
             /* Bit panel (§6.4, upstream style.css): a flat row of tnum
                chips completing the card with the bottom radius; the
                1 px top border separates it from the entry strip. Bits
                are flat buttons (no chip boxes, like the reference).*/
             .bit-panel { border-radius: 0 0 12px 12px; padding: 6px; border-top: 1px solid @card_shade_color; }
             /* Carousel-hosted bit panel (D25): a plain page, no card
                chrome — the card ends at the status row below 700 px.*/
             .bit-panel.in-carousel { border-top: none; border-radius: 0; padding: 12px 6px 0 6px; }
             .bit-toggle-button { padding: 1px; min-width: 9px; min-height: 0; font-feature-settings: \"tnum\"; }
             .bit-toggle-button:checked { color: @accent_color; }
             .bit-toggle-button.dim { opacity: 0.45; }
             .bit-toggle-button.bit-toggle-separator { margin-right: 10px; }
             .bit-marker-label { margin: 0 12px; font-feature-settings: \"tnum\"; }
             /* Converter (§7).*/
             .conv-card { padding: 12px 12px 0 12px; }
             .conv-card dropdown > button { background: none; }
             .conv-card dropdown > button:hover { background: alpha(currentColor, 0.08); }
             .conv-value { font-weight: 800; font-size: 181%; background: none; padding: 2px 10px; margin-bottom: 4px; font-feature-settings: \"tnum\"; }
             .conv-value > text { background: none; }
             /* Dialogs (§8.3–8.4).*/
             .char-dialog { border-radius: 18px; }
             .action-button { min-height: 24px; padding: 10px 20px; border-radius: 12px; }",
        );
        // Visual-harness hook: a bare Xvfb session has no settings portal,
        // so the accent is always the blue default; pin it to the reference
        // hue when comparing screenshots (never set in normal use).
        if let Ok(accent) = std::env::var("RADIX_DEBUG_ACCENT") {
            let accent_provider = gtk::CssProvider::new();
            accent_provider.load_from_string(&format!(
                "@define-color accent_color {accent}; @define-color accent_bg_color {accent};"
            ));
            gtk::style_context_add_provider_for_display(
                &gtk::gdk::Display::default().expect("no display"),
                &accent_provider,
                gtk::STYLE_PROVIDER_PRIORITY_THEME + 1,
            );
        }
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("no display"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    pub fn new(
        app: &gtk::Application,
        equation: Option<String>,
        start_mode: Option<ButtonMode>,
    ) -> Rc<Self> {
        Self::load_css();
        let settings = KeyfileStore::new().load();

        // Stateful menu actions: the mode and format radios are real Gio
        // state actions so the PopoverMenu renders radio dots and the
        // registered accelerators, like upstream (D25).
        let mode_action = gio::SimpleAction::new_stateful(
            "mode",
            Some(glib::VariantTy::STRING),
            &"basic".to_variant(),
        );
        let fmt_action = gio::SimpleAction::new_stateful(
            "number-format",
            Some(glib::VariantTy::STRING),
            &number_format_id(settings.number_format).to_variant(),
        );
        let clear_action = gio::SimpleAction::new("clear", None);

        // Startup size: persisted window-size wins (doc 06 §1.2); otherwise
        // per-mode defaults (doc 06 §2 / doc 13 §3.1: Basic/Conversion 360,
        // rest 700; default height ≈ the 616 px minimum).
        let (default_w, default_h) = if settings.window_size.0 > 0 && settings.window_size.1 > 0 {
            settings.window_size
        } else {
            let w = if matches!(
                settings.button_mode,
                ButtonMode::Basic | ButtonMode::Conversion
            ) {
                360
            } else {
                700
            };
            (w, 616)
        };
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Calculator")
            .default_width(default_w)
            .default_height(default_h)
            .build();
        if settings.window_maximized {
            window.maximize();
        }
        // Development builds carry the platform devel watermark (doc 13 §3.1).
        #[cfg(debug_assertions)]
        window.add_css_class("devel");
        window.set_icon_name(Some("io.github.hakergeniusz.Radix"));

        // --- header bar (doc 13 §3.2) ------------------------------------
        let header = adw::HeaderBar::new();
        // Mode menu (upstream math-window.blp): a real menu model on the
        // stateful `win.mode` action — radio dots, left-aligned labels and
        // the Ctrl+Alt+<mode> shortcut column come from the action group.
        let mode_btn = gtk::MenuButton::new();
        mode_btn.set_label("Basic");
        mode_btn.set_always_show_arrow(true);
        mode_btn.set_can_shrink(true);
        mode_btn.set_focus_on_click(false);
        mode_btn.add_css_class("flat");
        mode_btn.set_tooltip_text(Some(&crate::i18n::gettext("Mode Selection")));
        mode_btn.set_menu_model(Some(&mode_menu_model()));
        header.set_title_widget(Some(&mode_btn));

        let undo_btn = gtk::Button::from_icon_name("edit-undo-symbolic");
        let redo_btn = gtk::Button::from_icon_name("edit-redo-symbolic");
        for b in [&undo_btn, &redo_btn] {
            b.add_css_class("flat");
            b.set_focus_on_click(false);
            b.set_sensitive(false); // empty stacks at startup (doc 13 §3.2)
        }
        header.pack_start(&undo_btn);
        header.pack_start(&redo_btn);

        // Financial result-insert back navigation (doc 13 §3.2): hidden
        // except while a financial result is showing.
        let back_btn = gtk::Button::from_icon_name("go-previous-symbolic");
        back_btn.add_css_class("flat");
        back_btn.set_focus_on_click(false);
        back_btn.set_tooltip_text(Some(&crate::i18n::gettext("Financial")));
        back_btn.set_visible(false);
        header.pack_start(&back_btn);

        let menu_btn = gtk::MenuButton::new();
        menu_btn.set_icon_name("open-menu-symbolic");
        menu_btn.set_focus_on_click(false);
        menu_btn.add_css_class("flat");
        menu_btn.set_tooltip_text(Some(&crate::i18n::gettext("Main Menu")));
        // Primary menu (upstream primary_menu): sections with separators, a
        // "Result Format" header and accelerators for the wired actions.
        menu_btn.set_menu_model(Some(&primary_menu_model()));
        menu_btn.set_primary(true);
        header.pack_end(&menu_btn);

        // --- history area (doc 13 §4.2) ----------------------------------
        let history_box = gtk::ListBox::new();
        history_box.set_selection_mode(gtk::SelectionMode::None);
        // Rows hug the bottom of the history area, directly above the entry.
        history_box.set_valign(gtk::Align::End);
        let history_scroller = gtk::ScrolledWindow::new();
        history_scroller.set_child(Some(&history_box));
        history_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        history_scroller.set_vexpand(true);

        // --- display entry (doc 13 §4.3) ---------------------------------
        let display = Rc::new(RefCell::new(Display::new()));
        let tv = gtk::TextView::with_buffer(&display.borrow().buffer);
        tv.set_wrap_mode(gtk::WrapMode::None);
        tv.set_accepts_tab(false);
        tv.set_hexpand(true);
        tv.add_css_class("display-view");
        tv.set_direction(gtk::TextDirection::Ltr);
        // 14 px side text margins, 8 px above / 2 px below each line.
        tv.set_left_margin(14);
        tv.set_right_margin(14);
        tv.set_pixels_above_lines(8);
        tv.set_pixels_below_lines(2);
        tv.set_height_request(45);
        let tv_scroll = gtk::ScrolledWindow::new();
        tv_scroll.set_child(Some(&tv));
        tv_scroll.set_policy(gtk::PolicyType::External, gtk::PolicyType::Never);
        tv_scroll.set_max_content_height(64);

        // Circular flat backspace right of the entry (doc 13 §4.3).
        let backspace_btn = gtk::Button::from_icon_name("edit-clear-symbolic");
        backspace_btn.add_css_class("circular");
        backspace_btn.add_css_class("flat");
        backspace_btn.set_focus_on_click(false);
        backspace_btn.set_margin_start(6);
        backspace_btn.set_margin_end(6);
        backspace_btn.set_valign(gtk::Align::Center);

        let status = gtk::Label::new(None);
        status.set_halign(gtk::Align::Start);
        status.set_xalign(0.0);
        status.set_wrap(true);
        status.set_hexpand(true);
        status.add_css_class("status-view");
        let spinner = gtk::Spinner::new();
        spinner.set_visible(false);
        spinner.set_margin_start(12);
        spinner.set_valign(gtk::Align::Center);
        let mode_indicator = gtk::Label::new(None);
        mode_indicator.set_halign(gtk::Align::End);
        mode_indicator.set_margin_end(12);

        // Status/info row sits inside the entry area (doc 13 §4.3).
        let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        status_row.append(&spinner);
        status_row.append(&status);
        status_row.append(&mode_indicator);

        // The display card (doc 13 §4.1): history on top, entry below,
        // status inside the entry area; 12 px top/side margins, bottom 0.
        let entry_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        entry_row.append(&tv_scroll);
        entry_row.append(&backspace_btn);
        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("card");
        card.add_css_class("display-container");
        card.set_vexpand(true);
        let history_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
        history_area.add_css_class("history-view");
        history_area.set_vexpand(true);
        history_area.append(&history_scroller);
        card.append(&history_area);
        card.append(&entry_row);
        card.append(&status_row);

        // --- keypad stack -------------------------------------------------
        let keypad_stack = gtk::Stack::new();
        // No vexpand: the keypad pins to the bottom at its 236 px minimum
        // and the card/converter above absorbs the leftover height (the
        // reference keypads never stretch; a vexpand here made the
        // homogeneous rows grow instead).
        keypad_stack.set_transition_type(gtk::StackTransitionType::None);
        // Pages size independently: the wide modes' 700 px grids must not
        // leak their minimum into the 360 px basic/conversion pages.
        keypad_stack.set_hhomogeneous(false);
        keypad_stack.set_vhomogeneous(false);
        let keypad_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
        // Explicit false: the keypad must never take vertical extra space —
        // the card/converter above absorbs it. Without this, any child page
        // that computes expand=true (even a hidden one — expand flags leak
        // through the stack) stretches the homogeneous button rows.
        keypad_area.set_vexpand(false);
        keypad_area.set_margin_start(12);
        keypad_area.set_margin_end(12);
        keypad_area.set_margin_bottom(12);
        keypad_area.append(&keypad_stack);

        // --- layout: content column in the 700 px clamp (doc 13 §3.1) -----
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&card);
        content.append(&keypad_area);
        content.set_vexpand(true);
        let clamp = adw::Clamp::new();
        clamp.set_maximum_size(700);
        clamp.set_tightening_threshold(700);
        clamp.set_child(Some(&content));
        clamp.set_vexpand(true);

        // AdwApplicationWindow requires a ToolbarView: header as top bar,
        // layout as content (libadwaita replaces set_titlebar/set_child).
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_top_bar_style(adw::ToolbarStyle::Raised);
        toolbar.set_content(Some(&clamp));
        window.set_content(Some(&toolbar));
        window.set_size_request(360, 616);

        let win = Rc::new(CalcWindow {
            window: window.clone(),
            display,
            status,
            spinner,
            history_box,
            history_scroller,
            keypad_stack,
            mode_btn: mode_btn.clone(),
            menu_btn: menu_btn.clone(),
            undo_btn: undo_btn.clone(),
            redo_btn: redo_btn.clone(),
            back_btn: back_btn.clone(),
            backspace_btn: backspace_btn.clone(),
            card: card.clone(),
            content: content.clone(),
            tv: tv.clone(),
            mode_action,
            fmt_action,
            clear_action,
            settings: RefCell::new(settings),
            eval_state: Arc::new(Mutex::new(EvalState::default())),
            currencies: Arc::new(Mutex::new(CurrencyTable::new())),
            solve_rx: RefCell::new(None),
            solving: RefCell::new(false),
            solve_started: RefCell::new(None),
            history: RefCell::new(Vec::new()),
            last_equation: RefCell::new(None),
            base: RefCell::new(10),
            wordlen: RefCell::new(64),
            mode: RefCell::new(ButtonMode::Basic),
            inverse: RefCell::new(false),
            mode_indicator,
            prog: RefCell::new(None),
            advanced_pads: RefCell::new(None),
            financial_pads: RefCell::new(None),
            conv_pad: RefCell::new(None),
            built_pads: RefCell::new(Vec::new()),
            converter: RefCell::new(None),
            history_nav: RefCell::new(None),
            collapsed: std::cell::Cell::new(false),
            undo_sched: std::cell::Cell::new(false),
        });

        // Window actions (the "win." group): mode switching, clear history,
        // result format and preferences — the header menu models reference
        // them and app.rs registers their accelerators (D25).
        win.install_window_actions();

        // Keypad pages are built on first use (D32): apply_mode builds the
        // start mode synchronously below; the remaining pages are built on
        // the first idle, after the window has mapped. Keyboard mode has no
        // keypad (doc 06 §5.6) — an empty page.
        win.keypad_stack.add_named(
            &gtk::Box::new(gtk::Orientation::Vertical, 0),
            Some("keyboard"),
        );

        // Converter panel: a window-level card between the display card and
        // the keypad, visible only in conversion mode (doc 06 §6, doc 13 §7).
        let converter = ConverterPanel::new(&win);
        content.insert_child_after(&converter.root, Some(&card));
        converter.root.set_visible(false);
        *win.converter.borrow_mut() = Some(converter.clone());

        // Responsive collapse (doc 13 §6.1): below 700 px the wide grids
        // become swipeable carousel pages. adw::Breakpoint must stay (not a
        // width notify): the bin evaluates the condition BEFORE allocation,
        // so the wide layout's minimum no longer blocks shrinking to 360 px.
        // MaxWidth applies at ≤ the value; collapse is strictly below 700 px
        // (doc 13 §6.1), so the threshold sits 1 px lower.
        let bp = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            699.0,
            adw::LengthUnit::Px,
        ));
        {
            let w = Rc::downgrade(&win);
            bp.connect_apply(move |_| {
                if let Some(w) = w.upgrade() {
                    w.collapsed.set(true);
                    w.apply_collapse();
                }
            });
        }
        {
            let w = Rc::downgrade(&win);
            bp.connect_unapply(move |_| {
                if let Some(w) = w.upgrade() {
                    w.collapsed.set(false);
                    w.apply_collapse();
                }
            });
        }
        window.add_breakpoint(bp);

        // Undo/redo header buttons track the display stacks (doc 13 §3.2);
        // the buffer-changed signal fires inside a RefMut borrow, so the
        // refresh is deferred to an idle.
        {
            let w = Rc::downgrade(&win);
            win.display.borrow().buffer.connect_changed(move |_| {
                let Some(w) = w.upgrade() else { return };
                if w.undo_sched.replace(true) {
                    return;
                }
                let w2 = Rc::downgrade(&w);
                glib::idle_add_local_once(move || {
                    if let Some(w) = w2.upgrade() {
                        w.undo_sched.set(false);
                        let d = w.display.borrow();
                        w.undo_btn.set_sensitive(d.can_undo());
                        w.redo_btn.set_sensitive(d.can_redo());
                    }
                });
            });
        }

        // Wiring.
        win.connect_header();
        win.connect_display_keys(&tv);
        win.connect_global_keys();
        backspace_btn.connect_clicked({
            let d = Rc::clone(&win.display);
            move |_| d.borrow_mut().backspace()
        });
        back_btn.connect_clicked({
            let w = Rc::downgrade(&win);
            move |_| {
                if let Some(w) = w.upgrade() {
                    w.back_btn.set_visible(false);
                }
            }
        });

        win.apply_mode(start_mode.unwrap_or(win.settings.borrow().button_mode));
        win.apply_settings_to_serializer();
        win.restore_history();
        win.startup_currency_refresh();
        converter.apply_startup(&win);

        if let Some(eq) = equation {
            // Each non-empty line is placed into the display; empty lines
            // trigger a solve (doc 06 §1.6).
            for line in eq.split('\n') {
                if line.trim().is_empty() {
                    win.do_solve();
                } else {
                    win.display.borrow_mut().set_text(line);
                }
            }
        }

        // Ctrl+Q / menu Quit save-and-exit lives on each window (D31).
        crate::app::bind_close_all(&win.window);
        win.window.connect_close_request({
            let w = Rc::downgrade(&win);
            move |_| {
                // Close = move to background (D31): persist state and hide;
                // the process stays resident. Ctrl+Q or menu Quit exits.
                if let Some(w) = w.upgrade() {
                    w.save_shutdown_state();
                    w.window.set_visible(false);
                }
                glib::Propagation::Stop
            }
        });

        win.window.present();

        // Caret visible in the entry from the start (reference parity, D24).
        // grab_focus must come after present(): on an unmapped window it is
        // silently dropped, initial focus then lands on the first focusable
        // header widget and Return would activate the menu button (D25).
        tv.grab_focus();

        // Build the remaining keypad pages off the critical path (D32):
        // the first idle, after the window has mapped with the start
        // mode's page visible.
        {
            let w2 = Rc::downgrade(&win);
            glib::idle_add_local_once(move || {
                let Some(w) = w2.upgrade() else { return };
                for m in [
                    ButtonMode::Basic,
                    ButtonMode::Advanced,
                    ButtonMode::Financial,
                    ButtonMode::Programming,
                    ButtonMode::Conversion,
                ] {
                    w.ensure_pads(m);
                }
            });
        }

        win
    }

    // -----------------------------------------------------------------
    // Menus (doc 06 §3)
    // -----------------------------------------------------------------

    fn connect_header(self: &Rc<Self>) {
        let w = Rc::clone(self);
        self.undo_btn.connect_clicked(move |_| {
            w.display.borrow_mut().undo();
        });
        let w = Rc::clone(self);
        self.redo_btn.connect_clicked(move |_| {
            w.display.borrow_mut().redo();
        });
    }

    /// Window-scoped Gio actions backing the menus and accelerators
    /// (doc 06 §3, upstream math-window.vala): `win.mode`,
    /// `win.clear`, `win.number-format`, `win.preferences`.
    fn install_window_actions(self: &Rc<Self>) {
        let w = Rc::downgrade(self);
        self.mode_action.connect_activate(move |_, param| {
            let Some(w) = w.upgrade() else {
                return;
            };
            let mode = match param.and_then(|p| p.str()) {
                Some("advanced") => ButtonMode::Advanced,
                Some("financial") => ButtonMode::Financial,
                Some("programming") => ButtonMode::Programming,
                Some("keyboard") => ButtonMode::Keyboard,
                Some("conversion") => ButtonMode::Conversion,
                _ => ButtonMode::Basic,
            };
            w.apply_mode(mode);
        });
        self.window.add_action(&self.mode_action);

        let w = Rc::downgrade(self);
        self.clear_action.connect_activate(move |_, _| {
            if let Some(w) = w.upgrade() {
                w.clear_history_action();
            }
        });
        self.window.add_action(&self.clear_action);

        let w = Rc::downgrade(self);
        self.fmt_action.connect_activate(move |action, param| {
            let Some(w) = w.upgrade() else {
                return;
            };
            let fmt = match param.and_then(|p| p.str()) {
                Some("fixed") => NumberFormatSetting::Fixed,
                Some("scientific") => NumberFormatSetting::Scientific,
                Some("engineering") => NumberFormatSetting::Engineering,
                _ => NumberFormatSetting::Automatic,
            };
            w.settings.borrow_mut().number_format = fmt;
            w.apply_settings_to_serializer();
            action.set_state(&number_format_id(fmt).to_variant());
        });
        self.window.add_action(&self.fmt_action);

        let prefs = gio::SimpleAction::new("preferences", None);
        let w = Rc::downgrade(self);
        prefs.connect_activate(move |_, _| {
            if let Some(w) = w.upgrade() {
                crate::prefs::open_preferences(&w);
            }
        });
        self.window.add_action(&prefs);

        // Programming keypad menus (upstream cal.set-base/set-word-size).
        let prog_base = gio::SimpleAction::new("prog-base", Some(glib::VariantTy::STRING));
        let w = Rc::downgrade(self);
        prog_base.connect_activate(move |_, param| {
            let Some(w) = w.upgrade() else {
                return;
            };
            let base: u32 = param
                .and_then(|p| p.str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(10);
            w.dispatch_pad(PadAction::Base(base));
        });
        self.window.add_action(&prog_base);

        let prog_word = gio::SimpleAction::new("prog-word", Some(glib::VariantTy::STRING));
        let w = Rc::downgrade(self);
        prog_word.connect_activate(move |_, param| {
            let Some(w) = w.upgrade() else {
                return;
            };
            let word: u32 = param
                .and_then(|p| p.str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(64);
            w.dispatch_pad(PadAction::WordSize(word));
        });
        self.window.add_action(&prog_word);
    }

    pub fn clear_history_action(self: &Rc<Self>) {
        self.clear_history();
        self.settings.borrow_mut().history_tape.clear();
    }

    /// Wide ↔ carousel for the current mode's pads (doc 13 §6.1).
    fn apply_collapse(self: &Rc<Self>) {
        let collapsed = self.collapsed.get();
        // The `small` class drives the flush display-card margins/radius
        // below the 700 px clamp threshold (upstream clamp > .small, D24).
        if collapsed {
            self.content.add_css_class("small");
        } else {
            self.content.remove_css_class("small");
        }
        let prog = self.prog.borrow();
        if let Some(p) = prog.as_ref() {
            p.set_collapsed(collapsed, &self.card);
        }
        if *self.mode.borrow() == ButtonMode::Advanced {
            if let Some(p) = self.advanced_pads.borrow().as_ref() {
                p.set_collapsed(collapsed);
            }
        } else if matches!(*self.mode.borrow(), ButtonMode::Financial) {
            if let Some(p) = self.financial_pads.borrow().as_ref() {
                p.set_collapsed(collapsed);
            }
        }
        drop(prog);
        self.refocus_after_breakpoint();
    }

    /// The display keeps keyboard focus across breakpoint transitions
    /// (upstream never moves focus on a resize). While the bin hides its
    /// child for the transition frame, a focus walk cancels AdwBreakpointBin's
    /// own last-focus restore and lands on the first focusable header widget
    /// — whose Return then pops its menu (D25). Re-assert the entry focus on
    /// a 50 ms timeout: the bin's tick callback (which replays the walk)
    /// always runs within one frame, so the timeout is deterministic after
    /// it — an idle was not (it raced the replay).
    fn refocus_after_breakpoint(self: &Rc<Self>) {
        let w = Rc::downgrade(self);
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            let Some(w) = w.upgrade() else {
                return glib::ControlFlow::Break;
            };
            // An open popover owns the keyboard grab; don't fight it.
            if !w.menu_btn.is_active() && !w.mode_btn.is_active() {
                w.tv.grab_focus();
            }
            glib::ControlFlow::Break
        });
    }

    /// Build a mode's keypad page on first use (D32). apply_mode always
    /// calls this before set_visible_child_name; the start mode is built
    /// synchronously at startup, the remaining pages from the first idle.
    fn ensure_pads(self: &Rc<Self>, mode: ButtonMode) {
        if mode == ButtonMode::Keyboard || self.built_pads.borrow().contains(&mode) {
            return;
        }
        let w = Rc::clone(self);
        let sink: PadSink = Rc::new(move |a| w.dispatch_pad(a));
        match mode {
            ButtonMode::Basic => {
                let grid = keypad::basic(&sink);
                // No page scrolls (upstream parity): every page is a direct
                // child of the stack so its minimum height reaches the
                // window — a ScrolledWindow here swallowed the conversion
                // keypad's 236 px minimum and clipped its last rows.
                grid.set_margin_top(12);
                self.swap_pad_page("basic", &grid);
            }
            ButtonMode::Advanced => {
                let pads = keypad::advanced(&sink);
                pads.root.set_margin_top(12);
                let root = pads.root.clone();
                *self.advanced_pads.borrow_mut() = Some(pads);
                self.swap_pad_page("advanced", &root);
            }
            ButtonMode::Financial => {
                let pads = keypad::financial(&sink);
                pads.root.set_margin_top(12);
                let root = pads.root.clone();
                *self.financial_pads.borrow_mut() = Some(pads);
                self.swap_pad_page("financial", &root);
            }
            ButtonMode::Programming => {
                let pads = keypad::programming(&sink);
                pads.root.set_margin_top(12);
                // The bit panel lives inside the display card (doc 13 §6.4)
                // so its margins/radius follow the card's clamp mode (D24);
                // visibility is driven by apply_mode.
                self.card.append(&pads.bit_panel);
                pads.bit_panel.set_visible(false);
                let root = pads.root.clone();
                *self.prog.borrow_mut() = Some(pads);
                self.swap_pad_page("programming", &root);
            }
            ButtonMode::Conversion => {
                let pads = keypad::conversion(&sink);
                pads.grid.set_margin_top(12);
                let grid = pads.grid.clone();
                *self.conv_pad.borrow_mut() = Some(pads);
                self.swap_pad_page("conversion", &grid);
            }
            ButtonMode::Keyboard => {}
        }
        // Freshly built buttons must reflect the live modifier state.
        keypad::set_inverse_active(*self.inverse.borrow());
        let insert = self.display.borrow().insert_mode();
        keypad::set_mode_toggles(
            insert == InsertMode::Superscript,
            insert == InsertMode::Subscript,
        );
        self.attach_math_popovers();
        self.built_pads.borrow_mut().push(mode);
    }

    /// Replace a mode's stack page with its real keypad (D32).
    fn swap_pad_page<W: glib::object::IsA<gtk::Widget>>(&self, name: &'static str, widget: &W) {
        if let Some(old) = self.keypad_stack.child_by_name(name) {
            self.keypad_stack.remove(&old);
        }
        self.keypad_stack.add_named(widget, Some(name));
    }

    /// x / f(x) menu buttons: attach persistent popovers whose content
    /// refreshes on every open (doc 13 §5.2/§8.1). Runs after every keypad
    /// build; already-wired buttons are skipped.
    fn attach_math_popovers(self: &Rc<Self>) {
        keypad::for_each_math_button(|btn, kind| {
            if btn.popover().is_some() {
                return;
            }
            let pop = gtk::Popover::new();
            pop.add_css_class("math-popover");
            pop.set_position(gtk::PositionType::Top);
            let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
            content.set_size_request(320, 340);
            content.set_margin_top(6);
            content.set_margin_bottom(6);
            content.set_margin_start(6);
            content.set_margin_end(6);
            pop.set_child(Some(&content));
            btn.set_popover(Some(&pop));
            let w = Rc::downgrade(self);
            let pop2 = pop.clone();
            pop.connect_map(move |_| {
                let Some(w) = w.upgrade() else { return };
                crate::popovers::fill_math_popover(&content, &pop2, &w, kind);
            });
        });
    }

    pub fn apply_mode(self: &Rc<Self>, mode: ButtonMode) {
        let prev = *self.mode.borrow();
        *self.mode.borrow_mut() = mode;
        // The page must exist before set_visible_child_name (D32).
        self.ensure_pads(mode);
        let name = MODE_NAMES
            .iter()
            .find(|(m, _)| *m == mode)
            .map(|(_, n)| crate::i18n::gettext(n))
            .unwrap_or_else(|| "Basic".into());
        // Mode menu label + radio state (upstream mode_changed_cb sets the
        // menu button label; the stateful win.mode action drives the dots).
        self.mode_btn.set_label(&name);
        self.mode_action
            .set_state(&button_mode_id(mode).to_variant());
        let stack_name = match mode {
            ButtonMode::Basic => "basic",
            ButtonMode::Advanced => "advanced",
            ButtonMode::Financial => "financial",
            ButtonMode::Programming => "programming",
            ButtonMode::Keyboard => "keyboard",
            ButtonMode::Conversion => "conversion",
        };
        self.keypad_stack.set_visible_child_name(stack_name);
        // Keyboard mode hides the keypad entirely (doc 06 §5.6) and the
        // backspace button (doc 06 §4); the card gains a bottom margin and
        // fills the window (doc 13 §6.3).
        let keyboard = mode == ButtonMode::Keyboard;
        self.keypad_stack.set_visible(!keyboard);
        self.backspace_btn.set_visible(!keyboard);
        let content = self
            .card
            .parent()
            .and_then(|p| p.downcast::<gtk::Box>().ok());
        if let Some(content) = content {
            if keyboard {
                content.add_css_class("kb-mode");
            } else {
                content.remove_css_class("kb-mode");
            }
        }
        // Conversion mode replaces the display card with the converter panel
        // and hides undo/redo and history (doc 06 §5.5).
        let conversion = mode == ButtonMode::Conversion;
        self.card.set_visible(!conversion);
        self.history_scroller.set_visible(!conversion);
        self.undo_btn.set_visible(!conversion);
        self.redo_btn.set_visible(!conversion);
        // Clear History disables (and hides via hidden-when) in conversion
        // mode, like upstream mode_changed_cb (D25).
        self.clear_action.set_enabled(!conversion);
        if let Some(conv) = self.converter.borrow().as_ref() {
            conv.root.set_visible(conversion);
        }
        // Programming: square card bottom corners — the bit panel completes
        // the card (doc 13 §6.4) and rides inside it (D24).
        if mode == ButtonMode::Programming {
            self.card.add_css_class("prog");
        } else {
            self.card.remove_css_class("prog");
        }
        if let Some(p) = self.prog.borrow().as_ref() {
            p.bit_panel.set_visible(mode == ButtonMode::Programming);
        }
        // Default widths per mode (doc 06 §2): Basic/Conversion 360, rest 700.
        // Only on an actual mode change — startup must keep the restored
        // window-size (doc 06 §1.2).
        if mode != prev {
            self.window.set_default_width(
                if mode == ButtonMode::Basic || mode == ButtonMode::Conversion {
                    360
                } else {
                    700
                },
            );
        }
        self.apply_collapse();
        if mode == ButtonMode::Programming {
            *self.base.borrow_mut() = self.settings.borrow().base as u32;
            *self.wordlen.borrow_mut() = self.settings.borrow().word_size as u32;
            if let Some(p) = self.prog.borrow().as_ref() {
                p.set_base_sensitivity(*self.base.borrow());
            }
            self.update_word_label();
            self.update_base_label();
            self.update_bit_panel();
        } else {
            *self.base.borrow_mut() = 10;
        }
        self.apply_settings_to_serializer();
        self.display
            .borrow_mut()
            .rerender_answer(&self.serializer());
    } // -----------------------------------------------------------------
      // Serialization glue
      // -----------------------------------------------------------------

    pub fn serializer(&self) -> Serializer {
        let s = self.settings.borrow();
        let base = *self.base.borrow();
        Serializer {
            accuracy: s.accuracy.max(0) as u32,
            show_thousands: s.show_thousands,
            show_zeroes: s.show_zeroes,
            format: match s.number_format {
                NumberFormatSetting::Automatic => NumberFormat::Automatic,
                NumberFormatSetting::Fixed => NumberFormat::Fixed,
                NumberFormatSetting::Scientific => NumberFormat::Scientific,
                NumberFormatSetting::Engineering => NumberFormat::Engineering,
            },
            radix: '.',
            thousands: ',',
            repr_base: base,
            parser_base: base,
            ..Serializer::default()
        }
    }

    pub fn eval_config(&self) -> EvalConfig {
        let angle = match self.settings.borrow().angle_units {
            radix_core::settings::AngleSetting::Radians => AngleUnit::Radians,
            radix_core::settings::AngleSetting::Gradians => AngleUnit::Gradians,
            radix_core::settings::AngleSetting::Degrees => AngleUnit::Degrees,
        };
        EvalConfig {
            precision: self.settings.borrow().precision.max(0) as u32,
            wordlen: *self.wordlen.borrow(),
            angle_unit: angle,
            base: *self.base.borrow(),
        }
    }

    pub fn apply_settings_to_serializer(&self) {
        // Re-render an existing answer under the new settings (doc 06 §14).
        if self.display.borrow().has_answer() {
            self.display
                .borrow_mut()
                .rerender_answer(&self.serializer());
        }
    }

    // -----------------------------------------------------------------
    // Pad actions (doc 06 §5)
    // -----------------------------------------------------------------

    fn dispatch_pad(self: &Rc<Self>, action: PadAction) {
        // Conversion mode routes keypad input into the converter fields
        // (live conversion, no equals — doc 06 §5.5).
        if *self.mode.borrow() == ButtonMode::Conversion {
            let conv = self.converter.borrow().as_ref().map(Rc::clone);
            if let Some(conv) = conv {
                match action {
                    PadAction::Insert { text, .. } => {
                        conv.insert_text(&text);
                        self.clear_status();
                    }
                    PadAction::Backspace => conv.backspace(),
                    PadAction::Clear => conv.clear(),
                    PadAction::SwapUnits => conv.swap(self),
                    PadAction::Dms => conv.insert_dms(),
                    PadAction::RateDetails => {
                        let details = conv.rate_details(self);
                        if details.is_empty() {
                            self.set_status(
                                &crate::i18n::gettext(
                                    "Rate details available for currency conversions",
                                ),
                                false,
                            );
                        } else {
                            self.set_status(&details, false);
                        }
                    }
                    _ => {}
                }
                return;
            }
        }
        match action {
            PadAction::Insert { text, digit } => {
                self.display.borrow_mut().insert(&text, digit);
                self.clear_status();
            }
            PadAction::SmartBrackets => self.smart_bracket(),
            PadAction::Solve => self.do_solve(),
            PadAction::Clear => {
                self.display.borrow_mut().clear();
                self.clear_status();
            }
            PadAction::Backspace => self.display.borrow_mut().backspace(),
            PadAction::SupMode => {
                let next = if self.display.borrow().insert_mode() == InsertMode::Superscript {
                    InsertMode::Normal
                } else {
                    InsertMode::Superscript
                };
                self.display.borrow_mut().set_insert_mode(next);
                keypad::set_mode_toggles(next == InsertMode::Superscript, false);
                self.mode_indicator.set_text(match next {
                    InsertMode::Superscript => "superscript",
                    _ => "",
                });
            }
            PadAction::SubMode => {
                let next = if self.display.borrow().insert_mode() == InsertMode::Subscript {
                    InsertMode::Normal
                } else {
                    InsertMode::Subscript
                };
                self.display.borrow_mut().set_insert_mode(next);
                keypad::set_mode_toggles(false, next == InsertMode::Subscript);
                self.mode_indicator.set_text(match next {
                    InsertMode::Subscript => "subscript",
                    _ => "",
                });
            }
            PadAction::InverseToggle => {
                let next = !*self.inverse.borrow();
                *self.inverse.borrow_mut() = next;
                keypad::set_inverse_active(next);
            }
            PadAction::Factorize => self.factorize(),
            PadAction::AngleCycle => {
                // Deg→Rad→Grad (doc 06 §5.2).
                {
                    let mut st = self.settings.borrow_mut();
                    st.angle_units = match st.angle_units {
                        radix_core::settings::AngleSetting::Degrees => {
                            radix_core::settings::AngleSetting::Radians
                        }
                        radix_core::settings::AngleSetting::Radians => {
                            radix_core::settings::AngleSetting::Gradians
                        }
                        radix_core::settings::AngleSetting::Gradians => {
                            radix_core::settings::AngleSetting::Degrees
                        }
                    };
                }
                let label = match self.settings.borrow().angle_units {
                    radix_core::settings::AngleSetting::Radians => "Rad",
                    radix_core::settings::AngleSetting::Gradians => "Grad",
                    radix_core::settings::AngleSetting::Degrees => "Deg",
                };
                if let Some(page) = self.keypad_stack.visible_child() {
                    // Deep search: the page nests grids inside carousel
                    // pages/scrollviews (doc 13 §6.1).
                    let mut btns = Vec::new();
                    find_buttons_deep(&page, &mut btns);
                    for btn in btns {
                        if matches!(
                            btn.label().as_deref(),
                            Some("Deg") | Some("Rad") | Some("Grad")
                        ) {
                            btn.set_label(label);
                        }
                    }
                }
            }
            PadAction::Base(b) => {
                *self.base.borrow_mut() = b;
                if let Some(p) = self.prog.borrow().as_ref() {
                    p.set_base_sensitivity(b);
                }
                self.update_base_label();
                self.update_bit_panel();
                self.apply_settings_to_serializer();
            }
            PadAction::WordSize(w) => {
                *self.wordlen.borrow_mut() = w;
                self.settings.borrow_mut().word_size = w as i32;
                self.update_word_label();
                self.update_base_label();
                self.update_bit_panel();
            }
            PadAction::CharDialog => {
                crate::dialogs::open_character_dialog(&self.window, &Rc::clone(self));
            }
            PadAction::FinancialDialog(name) => {
                crate::dialogs::open_financial_dialog(&Rc::clone(self), name);
            }
            PadAction::ConversionView => {
                // Financial's "Currency Conversion" button: conversion view
                // with the Currency category preselected (doc 06 §5.4).
                if let Some(conv) = self.converter.borrow().as_ref() {
                    conv.select_category(self, "currency");
                }
                self.apply_mode(ButtonMode::Conversion);
            }
            PadAction::SwapUnits => { /* converter panel swap */ }
            PadAction::RateDetails => {}
            PadAction::Dms => {
                self.display.borrow_mut().insert("°", false);
            }
            PadAction::Bit(bit) => self.toggle_bit(bit),
        }
    }

    fn smart_bracket(self: &Rc<Self>) {
        let text = self.display.borrow().text();
        let cursor = self.display.borrow().buffer.cursor_position() as usize;
        let before = text.get(..cursor.min(text.len())).unwrap_or(&text);
        let opens = before.matches('(').count();
        let closes = before.matches(')').count();
        let last = before.chars().last();
        let after_value =
            matches!(last, Some(c) if c.is_ascii_digit() || c.is_alphabetic() || c == ')');
        let insert = if opens > closes && after_value {
            ")"
        } else {
            "("
        };
        self.display.borrow_mut().insert(insert, false);
    }

    fn toggle_bit(self: &Rc<Self>, bit: u32) {
        // Toggling acts on the current value, two's complement within the
        // word size for integers (doc 06 §5.3).
        let wordlen = *self.wordlen.borrow();
        if bit >= wordlen {
            return;
        }
        let Some(mut v) = self.parse_display_int() else {
            self.set_status(&crate::i18n::gettext("Bit editing needs an integer value"), true);
            return;
        };
        v ^= 1i128 << bit;
        // Two's complement wrap within the word size.
        if wordlen < 64 {
            let mask: i128 = (1i128 << wordlen) - 1;
            v &= mask;
        }
        let v64 = v as u64; // keeps the 64-bit two's complement pattern
        let ser = self.serializer();
        let out = ser.serialize(&Number::int(v64 as i64, self.eval_config().precision));
        match out {
            Ok(s) => {
                self.display.borrow_mut().set_text(&s);
                self.update_bit_panel();
            }
            Err(e) => self.set_status(&e.message, true),
        }
    }

    /// Current programming value: the last answer as an integer, else the
    /// display text parsed in the current base (two's complement for
    /// negatives via the u64 cast).
    fn parse_display_int(self: &Rc<Self>) -> Option<i128> {
        // An empty display has no value — the bit panel shows zeros and
        // greys out (upstream leaves the panel insensitive), D25.
        if self.display.borrow().text().trim().is_empty() {
            return None;
        }
        if let Some(v) = self.display.borrow().answer_value().cloned() {
            if let Some(i) = v.to_integer() {
                return Some(radix_core::units::int_of(i) as i128);
            }
        }
        let text = self.display.borrow().text();
        let base = *self.base.borrow();
        let clean: String = text
            .replace(['\u{202F}', ','], "")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if clean.is_empty() {
            return None;
        }
        let neg = clean.starts_with('−');
        let digits = if neg {
            &clean['−'.len_utf8()..]
        } else {
            &clean[..]
        };
        let v = i128::from_str_radix(digits, base).ok()?;
        Some(if neg { -v } else { v })
    }

    /// Bit panel sensitivity (word size) and float exponent-bit highlight
    /// (doc 06 §5.3: 52–62 accent + 63 dim for double; 23–30 + 31 for
    /// single precision).
    fn update_bit_panel(self: &Rc<Self>) {
        let wordlen = *self.wordlen.borrow();
        let prog = self.prog.borrow();
        let Some(prog) = prog.as_ref() else { return };
        let text = self.display.borrow().text();
        let is_float = text.contains('.');
        // Bit pattern: IEEE-754 double for floats (doc 06 §5.3 exponent
        // accents), else the integer value two's-complement within the
        // word size.
        let bits: u64 = if is_float {
            let from_answer = self
                .display
                .borrow()
                .answer_value()
                .and_then(|v| v.exact())
                .map(|r| r.to_f64());
            match from_answer {
                Some(f) => f.to_bits(),
                None => {
                    let clean: String = text
                        .replace(['\u{202F}', ','], "")
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect();
                    clean.parse::<f64>().map(|f| f.to_bits()).unwrap_or(0)
                }
            }
        } else {
            self.parse_display_int().map(|v| v as u64).unwrap_or(0)
        };
        // Buttons are stored bit 63 first in both arrangements (wide: two
        // 32-bit rows; narrow: four 16-bit rows — D25).
        for buttons in [&prog.bits, &prog.narrow_bits] {
            for (idx, btn) in buttons.iter().enumerate() {
                let bit = 63 - idx as u32;
                btn.set_sensitive(bit < wordlen);
                btn.remove_css_class("accent");
                btn.remove_css_class("dim");
                if is_float {
                    if wordlen == 64 && (52..=62).contains(&bit) {
                        btn.add_css_class("accent");
                    } else if wordlen == 64 && bit == 63 {
                        btn.add_css_class("dim");
                    } else if wordlen == 32 && (23..=30).contains(&bit) {
                        btn.add_css_class("accent");
                    } else if wordlen == 32 && bit == 31 {
                        btn.add_css_class("dim");
                    }
                }
                let on = bit < wordlen && (bits >> bit) & 1 == 1;
                btn.set_active(on);
                if let Some(lbl) = btn.child().and_then(|c| c.downcast::<gtk::Label>().ok()) {
                    lbl.set_text(if on { "1" } else { "0" });
                }
            }
        }
    }

    fn factorize(self: &Rc<Self>) {
        let text = self.display.borrow().canonicalize();
        let canonical = if let Some(_v) = self.display.borrow().answer_value().cloned() {
            if self.display.borrow().has_answer() && text.starts_with('_') {
                drop(text);
                return;
            }
            text
        } else {
            text
        };
        let config = self.eval_config();
        let units = radix_core::units::UnitCatalog::new();
        let currencies = self.currencies.lock().unwrap().clone();
        let mut state = self.eval_state.lock().unwrap().clone();
        let out =
            radix_core::solve::solve(&canonical, &config, &mut state, &units, &currencies, None);
        if let Some(v) = out.value {
            match radix_core::bitwise::factorize(&v) {
                Ok(factors) => {
                    let s = factors
                        .iter()
                        .map(|f| f.to_string())
                        .collect::<Vec<_>>()
                        .join("×");
                    self.display.borrow_mut().set_text(&s);
                }
                Err(e) => self.set_status(&e.message, true),
            }
        } else if let Some(e) = out.error {
            self.set_status(&e.message, true);
        }
    }

    // -----------------------------------------------------------------
    // Solve pipeline (doc 05 §1)
    // -----------------------------------------------------------------

    pub fn do_solve(self: &Rc<Self>) {
        if *self.solving.borrow() || *self.mode.borrow() == ButtonMode::Conversion {
            return; // busy, or live-conversion mode (no solve, doc 06 §5.5)
        }
        let display = self.display.borrow();
        let canonical = display.canonicalize();
        if canonical.trim().is_empty() {
            return; // empty display: no-op (doc 05 §2)
        }
        drop(display);

        // Repeat-last-op: `=` on a committed result re-solves the previous
        // equation with its last operator and right operand (doc 05 §1.5).
        let has_answer = self.display.borrow().has_answer();
        let mut expr = canonical.clone();
        if has_answer {
            if let Some(prev) = self.last_equation.borrow().as_ref() {
                expr = prev.clone();
            }
        }

        let rx = solver::spawn_solve(
            &expr,
            self.eval_config(),
            &self.eval_state,
            &self.currencies,
        );
        *self.solve_rx.borrow_mut() = Some(rx);
        *self.solving.borrow_mut() = true;
        *self.solve_started.borrow_mut() = Some(std::time::Instant::now());
        self.status.set_text("");
        self.status.remove_css_class("error");

        let w = Rc::downgrade(self);
        glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
            let Some(w) = w.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let reply = w.solve_rx.borrow_mut().as_ref().and_then(solver::poll);
            match reply {
                None => {
                    if w.solve_rx.borrow().is_some()
                        && w.solve_rx.borrow().as_ref().unwrap().try_recv().is_err()
                    {
                        // Keep polling; the spinner appears only after the
                        // 100 ms delay (doc 05 §1).
                        let pending = {
                            let rx = w.solve_rx.borrow();
                            rx.as_ref().map(|r| r.try_recv().is_err()).unwrap_or(false)
                        };
                        let elapsed = w
                            .solve_started
                            .borrow()
                            .map(|t| t.elapsed())
                            .unwrap_or_default();
                        if pending && elapsed >= std::time::Duration::from_millis(100) {
                            w.spinner.set_visible(true);
                            w.spinner.start();
                            w.status.set_text("Calculating…");
                        }
                        return glib::ControlFlow::Continue;
                    }
                    glib::ControlFlow::Break
                }
                Some(reply) => {
                    w.finish_solve(reply, expr.clone());
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn finish_solve(self: &Rc<Self>, reply: solver::SolveReply, equation: String) {
        *self.solving.borrow_mut() = false;
        self.solve_rx.borrow_mut().take();
        *self.solve_started.borrow_mut() = None;
        self.spinner.stop();
        self.spinner.set_visible(false);

        {
            let mut st = self.eval_state.lock().unwrap();
            *st = reply.new_state;
        }

        let outcome = reply.outcome;
        if let Some(e) = &outcome.error {
            // Status + error style + token selection (doc 06 §14).
            self.set_status(&e.message, true);
            if let (Some(s), Some(t)) = (e.token_start, e.token_end) {
                self.display.borrow_mut().select_range(s, t);
            }
            return;
        }
        let Some(v) = outcome.value else {
            return;
        };

        // Repeat-last-op data: the trailing binary operator of the solved
        // equation and its right operand (doc 05 §1.5).
        if let Some((op, rhs)) = trailing_binary_op(&equation) {
            let repeat = format!("{}{}{}", equation, op, rhs);
            *self.last_equation.borrow_mut() = Some(repeat);
        } else {
            *self.last_equation.borrow_mut() = None;
        }

        // A base conversion (`10 in hex`) overrides the display base for
        // this result (doc 02 §12.2; UNIT-T019).
        let mut ser = self.serializer();
        if outcome.repr_base != 0 {
            ser.repr_base = outcome.repr_base;
        }
        self.display.borrow_mut().show_result(&v, &ser);
        self.clear_status();

        // Programming info line + bit panel track the new value (doc 06 §5.3).
        if *self.mode.borrow() == ButtonMode::Programming {
            self.update_base_label();
            self.update_bit_panel();
        }

        // History entry (canonical equation with \cdot → ×, doc 05 §1.4).
        let answer = ser.serialize(&v).unwrap_or_default();
        let equation = equation.replace("\\cdot", "×");
        self.append_history(HistoryEntry { equation, answer });
        // The display keeps the caret after a solve (reference shows the
        // blinking cursor in the empty entry) — the = button would
        // otherwise hold focus.
        self.tv.grab_focus();
    }

    // -----------------------------------------------------------------
    // History (doc 06 §2, doc 05 §1.4)
    // -----------------------------------------------------------------

    fn append_history(self: &Rc<Self>, entry: HistoryEntry) {
        self.history.borrow_mut().push(entry.clone());
        // Three-part row (doc 13 §4.2): expression left (wraps, never
        // ellipsized), dimmed = centered, bold answer right (ellipsized,
        // max 12 chars).
        // Upstream row geometry (51.0 history-entry.blp): a 7-column
        // homogeneous grid — the equation spans 4 columns, the dimmed `=`
        // sits in its own column, the bold answer spans the last 2 — so a
        // narrow window squeezes the answer into an ellipsis ("0.8333…")
        // while a wide one shows it in full (D24).
        let row = gtk::Grid::new();
        row.set_column_homogeneous(true);
        row.set_margin_start(12);
        row.set_margin_end(16);
        row.add_css_class("history-entry");
        let eq = gtk::Label::new(Some(&entry.equation));
        eq.set_xalign(0.0);
        eq.set_wrap(true);
        eq.set_wrap_mode(pango::WrapMode::WordChar);
        eq.set_ellipsize(pango::EllipsizeMode::None);
        let equals = gtk::Label::new(Some("="));
        equals.add_css_class("dim-eq");
        equals.set_xalign(0.5);
        equals.set_max_width_chars(1);
        let ans = gtk::Label::new(Some(&entry.answer));
        ans.set_xalign(1.0);
        ans.set_ellipsize(pango::EllipsizeMode::End);
        ans.set_max_width_chars(12);
        ans.add_css_class("bold");
        row.attach(&eq, 0, 0, 4, 1);
        row.attach(&equals, 4, 0, 1, 1);
        row.attach(&ans, 5, 0, 2, 1);
        // Expression and answer each insert their text on click (doc 06 §2).
        for (lbl, text) in [(eq, entry.equation.clone()), (ans, entry.answer.clone())] {
            let gesture = gtk::GestureClick::new();
            let w = Rc::downgrade(self);
            gesture.connect_released(move |_, _, _, _| {
                if let Some(w) = w.upgrade() {
                    w.display.borrow_mut().insert(&text, false);
                    w.clear_status();
                    w.tv.grab_focus();
                }
            });
            lbl.add_controller(gesture);
        }
        let container = gtk::ListBoxRow::new();
        container.set_child(Some(&row));
        container.set_focusable(false);
        self.history_box.append(&container);
    }

    fn restore_history(self: &Rc<Self>) {
        let entries = self.settings.borrow().history_tape.clone();
        for e in entries {
            self.append_history(e);
        }
    }

    fn clear_history(self: &Rc<Self>) {
        self.history.borrow_mut().clear();
        while let Some(child) = self.history_box.first_child() {
            self.history_box.remove(&child);
        }
    }

    // -----------------------------------------------------------------
    // Status, base label, shutdown
    // -----------------------------------------------------------------

    fn set_status(&self, msg: &str, error: bool) {
        self.status.set_text(msg);
        if error {
            self.status.add_css_class("error");
        } else {
            self.status.remove_css_class("error");
        }
    }

    pub fn clear_status(&self) {
        self.status.set_text("");
        self.status.remove_css_class("error");
    }

    pub(crate) fn set_error_status(&self, msg: &str) {
        self.set_status(msg, true);
    }

    /// Financial dialog result: replaces the display and is not added to
    /// history (doc 04 §4). A value carrying a status error (Ddb's negative
    /// period) shows the value with the status-line error.
    pub(crate) fn show_financial_result(&self, v: radix_core::Number) {
        if let Some(e) = &v.err {
            self.set_status(&e.message, true);
        } else {
            self.clear_status();
        }
        // Result-insert navigation (doc 13 §3.2): the back button pops back
        // to the keypad while a financial result is showing.
        self.back_btn.set_visible(true);
        self.display
            .borrow_mut()
            .show_result(&v, &self.serializer());
    }

    fn base_name(base: u32) -> &'static str {
        match base {
            2 => "Binary",
            8 => "Octal",
            16 => "Hexadecimal",
            _ => "Decimal",
        }
    }

    fn update_base_label(&self) {
        // Info line: the current value in the bases other than the current
        // one, with subscript suffixes and an LTR mark (doc 06 §5.3);
        // negatives use the two's-complement magnitude in non-decimal bases
        // and a signed value in decimal. With no integer on display the
        // line is empty (upstream leaves equation.base_label blank).
        let base = *self.base.borrow();
        if let Some(p) = self.prog.borrow().as_ref() {
            p.base_btn.set_label(&crate::i18n::gettext(Self::base_name(base)));
        }
        let label = match self.parse_display_int_static() {
            Some(v) => {
                let wordlen = *self.wordlen.borrow();
                let mut parts: Vec<String> = Vec::new();
                for b in [2u32, 8, 10, 16] {
                    if b == base {
                        continue;
                    }
                    let magnitude: u128 = if v < 0 && b != 10 {
                        ((1i128 << wordlen.min(127)) + v) as u128
                    } else if v < 0 {
                        (-v) as u128
                    } else {
                        v as u128
                    };
                    let sign = if v < 0 && b == 10 { "−" } else { "" };
                    let sub: String = format!("{b}")
                        .chars()
                        .map(radix_core::serializer::subscript_digit)
                        .collect();
                    parts.push(format!("{}{}{}", sign, render_u128(magnitude, b), sub));
                }
                format!("\u{200E}{}", parts.join(" "))
            }
            None => String::new(),
        };
        self.mode_indicator.set_text(&label);
    }

    /// Programming mode's word-size menu button label (doc 06 §5.3).
    fn update_word_label(&self) {
        let w = *self.wordlen.borrow();
        if let Some(p) = self.prog.borrow().as_ref() {
            p.word_btn.set_label(&format!("{w}-bit"));
        }
    }

    /// Same as `parse_display_int` but borrows immutably (base label is
    /// called from non-Rc contexts).
    fn parse_display_int_static(&self) -> Option<i128> {
        // Empty display: no value (info line empty, bit panel zeroed, D25).
        if self.display.borrow().text().trim().is_empty() {
            return None;
        }
        if let Some(v) = self.display.borrow().answer_value().cloned() {
            if let Some(i) = v.to_integer() {
                return Some(radix_core::units::int_of(i) as i128);
            }
        }
        let text = self.display.borrow().text();
        let base = *self.base.borrow();
        let clean: String = text
            .replace(['\u{202F}', ','], "")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if clean.is_empty() {
            return None;
        }
        let neg = clean.starts_with('−');
        let digits = if neg {
            &clean['−'.len_utf8()..]
        } else {
            &clean[..]
        };
        let v = i128::from_str_radix(digits, base).ok()?;
        Some(if neg { -v } else { v })
    }

    pub(crate) fn save_shutdown_state(&self) {
        let mut settings = self.settings.borrow_mut();
        // History tape written per window close; mode/base/converter/window
        // size at shutdown of the last window (doc 06 §14, doc 08 §1/§3).
        settings.history_tape = self.history.borrow().clone();
        settings.button_mode = *self.mode.borrow();
        settings.base = *self.base.borrow() as i32;
        settings.word_size = *self.wordlen.borrow() as i32;
        let size = self.window.default_size();
        settings.window_size = (size.0, size.1);
        settings.window_maximized = self.window.is_maximized();
        // Converter panel: category and current category's unit pair
        // (doc 06 §6).
        if let Some(conv) = self.converter.borrow().as_ref() {
            conv.snapshot_settings(&mut settings);
        }
        drop(settings);
        KeyfileStore::new().save(&self.settings.borrow());

        // Registers and custom functions are re-saved on every change; a
        // final sync here keeps the files consistent.
        let state = self.eval_state.lock().unwrap();
        let _ = radix_core::persistence::save_registers(
            &radix_core::persistence::registers_path(),
            &state,
        );
        let _ = radix_core::persistence::save_functions(
            &radix_core::persistence::custom_functions_path(),
            &state,
        );
    }

    fn startup_currency_refresh(&self) {
        let rx =
            solver::spawn_rate_refresh(self.settings.borrow().refresh_interval, &self.currencies);
        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            if rx.try_recv().is_ok() {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    /// Re-fetch rates when the refresh-interval preference changes
    /// (doc 08: preferences change triggers an immediate re-refresh).
    pub fn restart_currency_refresh(&self) {
        self.startup_currency_refresh();
    }

    // -----------------------------------------------------------------
    // Keyboard entry (doc 06 §9.1)
    // -----------------------------------------------------------------

    fn connect_display_keys(self: &Rc<Self>, tv: &gtk::TextView) {
        let controller = gtk::EventControllerKey::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        let w = Rc::downgrade(self);
        controller.connect_key_pressed(move |_, keyval, _keycode, state| {
            let Some(w) = w.upgrade() else {
                return glib::Propagation::Proceed;
            };
            w.handle_key(keyval, state)
        });
        tv.add_controller(controller);
    }

    /// Window-level shortcuts (doc 06 §9.2): new/close window, quit,
    /// clear history, preferences, shortcuts dialog, help, clipboard,
    /// undo/redo, and Alt+Left/Right history navigation.
    fn connect_global_keys(self: &Rc<Self>) {
        let controller = gtk::EventControllerKey::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        let w = Rc::downgrade(self);
        controller.connect_key_pressed(move |_, key, _keycode, state| {
            let Some(w) = w.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let ctrl = state.contains(ModifierType::CONTROL_MASK);
            let alt = state.contains(ModifierType::ALT_MASK);
            let shift = state.contains(ModifierType::SHIFT_MASK);

            // Alt+Left/Right: history navigation (doc 06 §9.1).
            if alt && !ctrl && matches!(key, gtk::gdk::Key::Left | gtk::gdk::Key::Right) {
                let back = key == gtk::gdk::Key::Left;
                w.history_navigate(back);
                return glib::Propagation::Stop;
            }

            // Ctrl+digit → superscript digit; Alt+digit → subscript digit
            // (trailing spaces stripped first, doc 06 §9.1).
            if (ctrl ^ alt) && !shift {
                if let Some('0'..='9') = key.to_unicode() {
                    let c = key.to_unicode().unwrap();
                    let d = radix_core::serializer::superscript_digit(c).to_string();
                    let d = if alt {
                        radix_core::serializer::subscript_digit(c).to_string()
                    } else {
                        d
                    };
                    w.display.borrow_mut().strip_trailing_spaces();
                    w.display.borrow_mut().insert(&d, false);
                    return glib::Propagation::Stop;
                }
            }

            if ctrl && !alt {
                match key.to_unicode() {
                    Some('c') => {
                        w.copy_selection(false);
                        return glib::Propagation::Stop;
                    }
                    Some('v') => {
                        w.paste_clipboard();
                        return glib::Propagation::Stop;
                    }
                    Some('z') => {
                        if shift {
                            w.display.borrow_mut().redo();
                        } else {
                            w.display.borrow_mut().undo();
                        }
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
                if key == gtk::gdk::Key::Delete {
                    // Ctrl+Delete clears the display (doc 06 §9.1).
                    w.display.borrow_mut().clear();
                    w.clear_status();
                    return glib::Propagation::Stop;
                }
                // Ctrl+N / Ctrl+W / Ctrl+, / Ctrl+? / Ctrl+Escape and F1 are
                // registered accelerators on app actions now (app.rs, D25);
                // they must fall through to the accel group.
            }

            if !ctrl && !alt && !shift {
                match key {
                    gtk::gdk::Key::F10 => {
                        // F10 opens the primary menu (doc 06 §9.2).
                        w.menu_btn.popup();
                        return glib::Propagation::Stop;
                    }
                    // Hardware Copy/Paste/Undo/Redo keys (doc 06 §9.2).
                    gtk::gdk::Key::Copy => {
                        w.copy_selection(false);
                        return glib::Propagation::Stop;
                    }
                    gtk::gdk::Key::Paste => {
                        w.paste_clipboard();
                        return glib::Propagation::Stop;
                    }
                    gtk::gdk::Key::Undo => {
                        w.display.borrow_mut().undo();
                        return glib::Propagation::Stop;
                    }
                    gtk::gdk::Key::Redo => {
                        w.display.borrow_mut().redo();
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(controller);
    }

    /// Copy with separator stripping; the whole expression when nothing is
    /// selected (doc 06 §9.2, §11).
    fn copy_selection(&self, _strip_all: bool) {
        let d = self.display.borrow();
        let (start, end) = match d.selection_bounds() {
            Some((s, e)) => (s, e),
            None => (0, d.buffer.char_count()),
        };
        let text = d.text();
        let s = text
            .chars()
            .skip(start as usize)
            .take((end - start).max(0) as usize)
            .collect::<String>()
            .replace(['\u{202F}', ','], "");
        drop(d);
        gtk::prelude::RootExt::display(&self.window)
            .clipboard()
            .set_text(&s);
    }

    /// Paste with newlines collapsed to spaces (doc 06 §11).
    fn paste_clipboard(&self) {
        let d = Rc::clone(&self.display);
        gtk::prelude::RootExt::display(&self.window)
            .clipboard()
            .read_text_async(
                gio::Cancellable::NONE,
                move |res: Result<Option<glib::GString>, glib::Error>| {
                    if let Ok(Some(t)) = res {
                        let cleaned = t.replace(['\n', '\r'], " ");
                        d.borrow_mut().insert(&cleaned, false);
                    }
                },
            );
    }

    /// Alt+Left/Right steps through history equations (doc 06 §9.1).
    fn history_navigate(self: &Rc<Self>, back: bool) {
        let len = self.history.borrow().len();
        if len == 0 {
            return;
        }
        let mut nav = self.history_nav.borrow_mut();
        let next = match *nav {
            None => {
                if back {
                    len - 1
                } else {
                    return;
                }
            }
            Some(i) => {
                if back {
                    if i == 0 {
                        return;
                    }
                    i - 1
                } else if i + 1 >= len {
                    // Forward past the newest entry restores an empty display.
                    *nav = None;
                    self.display.borrow_mut().clear();
                    self.clear_status();
                    return;
                } else {
                    i + 1
                }
            }
        };
        *nav = Some(next);
        let eq = self.history.borrow()[next].equation.clone();
        self.display.borrow_mut().set_text(&eq);
    }

    fn handle_key(self: &Rc<Self>, key: gtk::gdk::Key, state: ModifierType) -> glib::Propagation {
        let ctrl = state.contains(ModifierType::CONTROL_MASK);
        let alt = state.contains(ModifierType::ALT_MASK);
        let _shift = state.contains(ModifierType::SHIFT_MASK);
        let unicode = key.to_unicode();

        // Mode accelerators Ctrl+Alt+* (doc 06 §9.3).
        if ctrl && alt {
            let mode = match key {
                gtk::gdk::Key::b => Some(ButtonMode::Basic),
                gtk::gdk::Key::a => Some(ButtonMode::Advanced),
                gtk::gdk::Key::f => Some(ButtonMode::Financial),
                gtk::gdk::Key::p => Some(ButtonMode::Programming),
                gtk::gdk::Key::k | gtk::gdk::Key::t => Some(ButtonMode::Keyboard),
                gtk::gdk::Key::c => Some(ButtonMode::Conversion),
                _ => None,
            };
            if let Some(m) = mode {
                self.apply_mode(m);
                return glib::Propagation::Stop;
            }
        }

        // Programming base shortcuts (doc 06 §9.1).
        if ctrl && *self.mode.borrow() == ButtonMode::Programming {
            let base = match unicode {
                Some('b') => Some(2u32),
                Some('o') => Some(8),
                Some('d') => Some(10),
                Some('h') => Some(16),
                _ => None,
            };
            if let Some(b) = base {
                self.dispatch_pad(PadAction::Base(b));
                return glib::Propagation::Stop;
            }
        }

        // Ctrl shortcuts (doc 06 §9.1).
        if ctrl && !alt {
            let insert = match unicode {
                Some('r') => Some("√"),
                Some('p') => Some("π"),
                Some('t') => Some("τ"),
                Some('i') => Some("⁻¹"),
                Some('e') => {
                    self.display
                        .borrow_mut()
                        .set_insert_mode(InsertMode::Superscript);
                    keypad::set_mode_toggles(true, false);
                    self.mode_indicator.set_text("superscript");
                    return glib::Propagation::Stop;
                }
                Some('f') => {
                    self.factorize();
                    return glib::Propagation::Stop;
                }
                Some('o') => Some("°"),
                Some('u') => Some("µ"),
                Some('\u{2013}') | Some('-') => Some("⁻"),
                _ => None,
            };
            let insert = insert.or(match key {
                gtk::gdk::Key::bracketleft => Some("⌈"),
                gtk::gdk::Key::bracketright => Some("⌉"),
                _ => None,
            });
            if let Some(s) = insert {
                self.display.borrow_mut().insert(s, false);
                self.clear_status();
                return glib::Propagation::Stop;
            }
        }

        // Alt shortcuts.
        if alt && !ctrl {
            let insert = match key {
                gtk::gdk::Key::bracketleft => Some("⌊"),
                gtk::gdk::Key::bracketright => Some("⌋"),
                _ => None,
            };
            if let Some(s) = insert {
                self.display.borrow_mut().insert(s, false);
                return glib::Propagation::Stop;
            }
        }

        // Hex letters insert directly in hex base (doc 06 §9.1).
        if *self.base.borrow() == 16 && !ctrl && !alt {
            if let Some(c) = unicode {
                if ('a'..='f').contains(&c) || ('A'..='F').contains(&c) {
                    self.display
                        .borrow_mut()
                        .insert(&c.to_uppercase().to_string(), true);
                    return glib::Propagation::Stop;
                }
            }
        }

        // Substitution keys and digits (doc 06 §9.1).
        let text: Option<(String, bool)> = if let Some(c) = unicode {
            match c {
                '0'..='9' => Some((c.to_string(), true)),
                '*' => {
                    // `×` after `×` becomes `^` (doc 06 §9.1 substitutions).
                    let cur = self.display.borrow().text();
                    if cur.ends_with('×') {
                        drop(cur);
                        let mut d = self.display.borrow_mut();
                        d.backspace();
                        d.insert("^", false);
                    } else {
                        drop(cur);
                        self.display.borrow_mut().insert("×", false);
                    }
                    self.clear_status();
                    return glib::Propagation::Stop;
                }
                '/' => Some(("÷".into(), false)),
                '-' => Some(("−".into(), false)),
                '+' => Some(("+".into(), false)),
                '\'' => Some(("′".into(), false)),
                '"' => Some(("″".into(), false)),
                '%' => Some(("%".into(), false)),
                '!' => Some(("!".into(), false)),
                '^' => Some(("^".into(), false)),
                '(' => Some(("(".into(), false)),
                // The radix point must route through insert() like the keypad
                // dot (undo snapshots, fresh-start after an answer) — a fall
                // through would let the TextView insert it natively.
                '.' => Some((".".into(), false)),
                '>' => {
                    self.chain_shift("≫", "⋙");
                    return glib::Propagation::Stop;
                }
                '<' => {
                    self.chain_shift("≪", "≪");
                    return glib::Propagation::Stop;
                }
                _ => None,
            }
        } else {
            match key {
                gtk::gdk::Key::KP_Multiply => Some(("×".into(), false)),
                gtk::gdk::Key::KP_Divide => Some(("÷".into(), false)),
                gtk::gdk::Key::KP_Subtract => Some(("−".into(), false)),
                gtk::gdk::Key::KP_Add => Some(("+".into(), false)),
                gtk::gdk::Key::KP_Decimal => Some((".".into(), false)),
                _ => {
                    // KP digits arrive by name with NumLock off (doc 06 §9.1).
                    let name = key.name().unwrap_or_default();
                    if let Some(d) = name.strip_prefix("KP_") {
                        if d.len() == 1 && d.chars().all(|c| c.is_ascii_digit()) {
                            return glib::Propagation::Stop;
                        }
                    }
                    None
                }
            }
        };
        if let Some((s, digit)) = text {
            self.display.borrow_mut().insert(&s, digit);
            self.clear_status();
            return glib::Propagation::Stop;
        }

        match key {
            gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                self.do_solve();
                glib::Propagation::Stop
            }
            gtk::gdk::Key::equal | gtk::gdk::Key::KP_Equal => {
                if self.display.borrow().is_name_or_header() {
                    self.display.borrow_mut().insert("=", false);
                } else {
                    self.do_solve();
                }
                glib::Propagation::Stop
            }
            gtk::gdk::Key::Escape => {
                self.display.borrow_mut().clear();
                self.clear_status();
                glib::Propagation::Stop
            }
            gtk::gdk::Key::BackSpace => {
                self.display.borrow_mut().backspace();
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    }

    fn chain_shift(self: &Rc<Self>, once: &str, twice: &str) {
        // `>` after `≫` becomes ⋙; a first `>` inserts ≫ (doc 06 §9.1).
        let mut d = self.display.borrow_mut();
        let text = d.text();
        let cursor = d.buffer.cursor_position() as usize;
        let before = text.get(..cursor.min(text.len())).unwrap_or(&text);
        if before.ends_with(once) {
            d.backspace();
            d.insert(twice, false);
        } else {
            d.insert(once, false);
        }
    }
}

// ---------------------------------------------------------------------
// Menus (upstream math-window.blp menu models, D25)
// ---------------------------------------------------------------------

/// Mode-menu model: one radio section on the stateful `win.mode` action.
fn mode_menu_model() -> gio::Menu {
    let section = gio::Menu::new();
    for (mode, name) in MODE_NAMES {
        let item = gio::MenuItem::new(Some(&crate::i18n::gettext(name)), None);
        item.set_action_and_target_value(
            Some("win.mode"),
            Some(&button_mode_id(mode).to_variant()),
        );
        section.append_item(&item);
    }
    let menu = gio::Menu::new();
    menu.append_section(None, &section);
    menu
}

/// Primary-menu model: New Window | Clear History | Result Format section |
/// help section (Preferences, Keyboard Shortcuts, Help, About).
fn primary_menu_model() -> gio::Menu {
    let menu = gio::Menu::new();

    let s = gio::Menu::new();
    s.append(Some(&crate::i18n::gettext("New Window")), Some("app.new-window"));
    menu.append_section(None, &s);

    let s = gio::Menu::new();
    let clear = gio::MenuItem::new(Some(&crate::i18n::gettext("Clear History")), Some("win.clear"));
    // Hidden while the action is disabled (conversion mode), like upstream.
    clear.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
    s.append_item(&clear);
    menu.append_section(None, &s);

    let s = gio::Menu::new();
    for (target, label) in [
        ("automatic", "Automatic"),
        ("fixed", "Fixed"),
        ("scientific", "Scientific"),
        ("engineering", "Engineering"),
    ] {
        let item = gio::MenuItem::new(Some(&crate::i18n::gettext(label)), None);
        item.set_action_and_target_value(Some("win.number-format"), Some(&target.to_variant()));
        s.append_item(&item);
    }
    menu.append_section(Some(&crate::i18n::gettext("Result Format")), &s);

    let s = gio::Menu::new();
    s.append(Some(&crate::i18n::gettext("Preferences")), Some("win.preferences"));
    s.append(Some(&crate::i18n::gettext("Keyboard Shortcuts")), Some("app.shortcuts"));
    s.append(Some(&crate::i18n::gettext("Help")), Some("app.help"));
    s.append(Some(&crate::i18n::gettext("About Calculator")), Some("app.about"));
    menu.append_section(None, &s);

    // Closing the window moves the app to the background (D31), so the
    // menu carries the real exit.
    let s = gio::Menu::new();
    s.append(Some(&crate::i18n::gettext("Quit")), Some("win.close-all"));
    menu.append_section(None, &s);
    menu
}

/// `win.mode` action/accelerator id for a button mode.
fn button_mode_id(mode: ButtonMode) -> &'static str {
    match mode {
        ButtonMode::Basic => "basic",
        ButtonMode::Advanced => "advanced",
        ButtonMode::Financial => "financial",
        ButtonMode::Programming => "programming",
        ButtonMode::Keyboard => "keyboard",
        ButtonMode::Conversion => "conversion",
    }
}

/// `win.number-format` action id for a format setting.
fn number_format_id(fmt: NumberFormatSetting) -> &'static str {
    match fmt {
        NumberFormatSetting::Automatic => "automatic",
        NumberFormatSetting::Fixed => "fixed",
        NumberFormatSetting::Scientific => "scientific",
        NumberFormatSetting::Engineering => "engineering",
    }
}

fn children_of(widget: &gtk::Widget) -> Vec<gtk::Widget> {
    let mut out = Vec::new();
    let mut cur = widget.first_child();
    while let Some(c) = cur {
        let next = c.next_sibling();
        out.push(c);
        cur = next;
    }
    out
}

/// Depth-first collection of every Button under `widget`.
fn find_buttons_deep(widget: &gtk::Widget, out: &mut Vec<gtk::Button>) {
    if let Some(b) = widget.downcast_ref::<gtk::Button>() {
        out.push(b.clone());
        return; // button children are labels, not buttons
    }
    for c in children_of(widget) {
        find_buttons_deep(&c, out);
    }
}

/// Render a non-negative integer in `base` (base label helper).
fn render_u128(mut v: u128, base: u32) -> String {
    if v == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while v > 0 {
        out.push(char::from_digit((v % base as u128) as u32, base).unwrap_or('0'));
        v /= base as u128;
    }
    out.into_iter().rev().collect()
}

/// Trailing repeatable binary operator + right operand of a canonical
/// equation (doc 05 §1.5: + − × ÷ and shifts repeat).
fn trailing_binary_op(equation: &str) -> Option<(String, String)> {
    const OPS: [&str; 7] = [" ≪ ", " ≫ ", " ⋙ ", "+", "−", "×", "÷"];
    let mut depth = 0i32;
    let mut best: Option<(usize, &str)> = None;
    let chars: Vec<char> = equation.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if depth != 0 {
            continue;
        }
        for op in OPS {
            let oc: Vec<char> = op.chars().collect();
            if i >= oc.len() && chars[i + 1 - oc.len()..i + 1] == oc[..] {
                best = Some((i + 1 - oc.len(), op));
            }
        }
    }
    let (start, op) = best?;
    let rhs: String = chars[start + op.chars().count()..].iter().collect();
    if rhs.trim().is_empty() {
        return None;
    }
    Some((op.to_string(), rhs))
}
