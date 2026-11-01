//! Keypad panels (doc 06 §5; visual styling per doc 13 §5–6). Buttons never
//! take focus on click; every control reports a [`PadAction`] through the
//! shared callback and the window decides what it means (insertion, solving,
//! dialogs, modes). Wide modes are built as two 5-column grids that sit side
//! by side at ≥ 700 px and collapse into a swipeable carousel below
//! (doc 13 §6.1).

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::{Button, Grid, Label, Orientation, ToggleButton};
use std::cell::Cell;
use std::rc::Rc;

/// Actions a keypad button can request (resolved by the window).
#[derive(Debug, Clone)]
pub enum PadAction {
    /// Insert text at the cursor (`digit` for digit-class inserts, which
    /// honor super/subscript modes).
    Insert {
        text: String,
        digit: bool,
    },
    Solve,
    Clear,
    Backspace,
    /// Toggle superscript (↑n) / subscript (↓n) entry modes.
    SupMode,
    SubMode,
    /// ⇧⁻¹ inverse modifier toggle.
    InverseToggle,
    /// a×b — factorize the displayed value.
    Factorize,
    /// Angle-unit cycle button (Deg→Rad→Grad); label via `AngleCycle`.
    AngleCycle,
    /// Base selector (programming).
    Base(u32),
    /// Word-size selector (programming).
    WordSize(u32),
    /// Character dialog (á, programming).
    CharDialog,
    /// Financial dialog openers (doc 04 §4).
    FinancialDialog(&'static str),
    /// Currency Conversion button (financial) → conversion view.
    ConversionView,
    /// ⇅ swap units (conversion keypad).
    SwapUnits,
    /// Rate details (conversion keypad, currency category).
    RateDetails,
    /// °′″ DMS insertion (conversion keypad, angle category).
    Dms,
    /// Bit-panel toggle (programming), bit index.
    Bit(u32),
    /// `( )` smart brackets — the window decides `(` or `)` (doc 06 §5).
    SmartBrackets,
}

pub type PadSink = Rc<dyn Fn(PadAction)>;

// ---------------------------------------------------------------------
// Button constructors (doc 13 §5.2 style classes)
// ---------------------------------------------------------------------

fn connect_action(b: &Button, action: PadAction, sink: &PadSink) {
    b.set_focusable(false);
    let sink = Rc::clone(sink);
    b.connect_clicked(move |_| sink(action.clone()));
}

/// Default keypad button (base style, no class override).
fn button(label: &str, action: PadAction, sink: &PadSink) -> Button {
    let b = Button::with_label(label);
    connect_action(&b, action, sink);
    b
}

fn classed(label: &str, class: &str, action: PadAction, sink: &PadSink) -> Button {
    let b = button(label, action, sink);
    b.add_css_class(class);
    b
}

/// Digits and hex digits (doc 13 §5.2: 1.2 em bolder, `alpha(currentColor,.2)`).
fn num_button(label: &str, action: PadAction, sink: &PadSink) -> Button {
    classed(label, "number-button", action, sink)
}

/// ÷ × − + (doc 13 §5.2: 1.6 em glyph).
fn op_button(label: &str, action: PadAction, sink: &PadSink) -> Button {
    classed(label, "operator-button", action, sink)
}

/// mod, ↑n, ↓n, trig, bit ops and all other extras (hook class).
fn fn_button(label: &str, action: PadAction, sink: &PadSink) -> Button {
    classed(label, "function-button", action, sink)
}

/// The C button (doc 13 §5.2: 1.2 em bolder).
fn clear_button(action: PadAction, sink: &PadSink) -> Button {
    classed("C", "clear-button", action, sink)
}

/// The = button (doc 13 §5.2: accent-filled 1.6 em).
fn eq_button(sink: &PadSink) -> Button {
    classed("=", "suggested-action", PadAction::Solve, sink)
}

/// The radix point (hook class).
fn point_button(action: PadAction, sink: &PadSink) -> Button {
    classed(".", "numeric-point-button", action, sink)
}

/// Smart brackets / plain parens (hook class).
fn paren_button(label: &str, action: PadAction, sink: &PadSink) -> Button {
    classed(label, "parenthesis-button", action, sink)
}

/// Percent (hook class).
fn pct_button(action: PadAction, sink: &PadSink) -> Button {
    classed("%", "percent-button", action, sink)
}

fn attach(grid: &Grid, w: &impl IsA<gtk::Widget>, col: i32, row: i32, ws: i32, hs: i32) {
    grid.attach(w, col, row, ws, hs);
}

/// Button with Pango markup content (`logᵧ`, doc 06 §5.2).
fn markup_button(markup: &str, action: PadAction, sink: &PadSink) -> Button {
    let b = Button::new();
    let lbl = Label::new(None);
    lbl.set_markup(markup);
    b.set_child(Some(&lbl));
    connect_action(&b, action, sink);
    b
}

fn math_grid() -> Grid {
    let grid = Grid::new();
    grid.set_row_spacing(4);
    grid.set_column_spacing(4);
    // Fill the viewport with equal cells (upstream panels: hexpand +
    // row/column-homogeneous; the CSS min sizes keep the 336×236 block).
    grid.set_hexpand(true);
    grid.set_row_homogeneous(true);
    grid.set_column_homogeneous(true);
    grid.add_css_class("math-buttons");
    grid
}

// ---------------------------------------------------------------------
// Superscript/subscript and inverse-modifier toggle state
// ---------------------------------------------------------------------

thread_local! {
    static MODE_TOGGLES: std::cell::RefCell<Vec<(glib::WeakRef<ToggleButton>, u8)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn mode_toggle(label: &'static str, which: u8, action: PadAction, sink: &PadSink) -> ToggleButton {
    let t = ToggleButton::with_label(label);
    t.set_focusable(false);
    t.add_css_class("function-button");
    let sink = Rc::clone(sink);
    t.connect_clicked(move |_| sink(action.clone()));
    MODE_TOGGLES.with(|list| {
        let weak = glib::WeakRef::new();
        weak.set(Some(&t));
        list.borrow_mut().push((weak, which));
    });
    t
}

/// Reflect the active insert mode on the ↑n/↓n toggles (doc 13 §5.2:
/// checked state marks the active modifier).
pub fn set_mode_toggles(sup: bool, sub: bool) {
    MODE_TOGGLES.with(|list| {
        for (weak, which) in list.borrow().iter() {
            if let Some(t) = weak.upgrade() {
                t.set_active(match which {
                    0 => sup,
                    _ => sub,
                });
            }
        }
    });
}

/// A button whose label/action depends on the inverse modifier.
struct InversePair {
    normal: (&'static str, PadAction),
    inverse: (&'static str, PadAction),
}

fn inverse_pair_button(pair: InversePair, css: &str, sink: &PadSink) -> Button {
    let b = Button::new();
    // Markup label: the log pairs carry sub/superscripts (log<sub>y</sub>).
    let lbl = Label::new(None);
    lbl.set_markup(pair.normal.0);
    b.set_child(Some(&lbl));
    b.set_focusable(false);
    b.add_css_class(css);
    let state = Rc::new(std::cell::Cell::new(false));
    let actions = Rc::new((pair.normal.1, pair.inverse.1));
    let labels = Rc::new((pair.normal.0, pair.inverse.0));
    let sink = Rc::clone(sink);
    let st = Rc::clone(&state);
    let acts = Rc::clone(&actions);
    b.connect_clicked(move |_| {
        let a = if st.get() {
            acts.1.clone()
        } else {
            acts.0.clone()
        };
        sink(a);
    });
    // Registered for toggling via the window's inverse state broadcast.
    INVERSE_BUTTONS.with(|list| {
        list.borrow_mut().push(InverseButton {
            widget: b.downgrade(),
            labels,
            state: Rc::clone(&state),
        });
    });
    b
}

pub struct InverseButton {
    pub widget: gtk::glib::object::WeakRef<Button>,
    labels: Rc<(&'static str, &'static str)>,
    state: Rc<std::cell::Cell<bool>>,
}

thread_local! {
    static INVERSE_BUTTONS: std::cell::RefCell<Vec<InverseButton>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static INVERSE_MODS: std::cell::RefCell<Vec<glib::WeakRef<ToggleButton>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// ⇧⁻¹ toggle buttons (accent style per doc 13 §5.2).
fn inverse_mod_button(sink: &PadSink) -> ToggleButton {
    let t = ToggleButton::with_label("⇧⁻¹");
    t.set_focusable(false);
    t.add_css_class("function-button");
    t.add_css_class("accent");
    let sink = Rc::clone(sink);
    t.connect_clicked(move |_| sink(PadAction::InverseToggle));
    INVERSE_MODS.with(|list| {
        let weak = glib::WeakRef::new();
        weak.set(Some(&t));
        list.borrow_mut().push(weak);
    });
    t
}

/// Update every inverse-dependent button to the given modifier state
/// (doc 06 §5.2: ⇧⁻¹ toggle or holding Shift).
pub fn set_inverse_active(active: bool) {
    INVERSE_BUTTONS.with(|list| {
        for b in list.borrow().iter() {
            b.state.set(active);
            if let Some(w) = b.widget.upgrade() {
                let text = if active { b.labels.1 } else { b.labels.0 };
                if let Some(l) = w.child().and_then(|c| c.downcast::<Label>().ok()) {
                    l.set_markup(text);
                } else {
                    w.set_label(text);
                }
            }
        }
    });
    INVERSE_MODS.with(|list| {
        for weak in list.borrow().iter() {
            if let Some(t) = weak.upgrade() {
                t.set_active(active);
            }
        }
    });
}

// ---------------------------------------------------------------------
// x / f(x) menu buttons (doc 13 §5.2: MenuButtons with up carets)
// ---------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum MathPopoverKind {
    Variables,
    Functions,
}

thread_local! {
    static MATH_BUTTONS: std::cell::RefCell<Vec<(glib::WeakRef<gtk::MenuButton>, MathPopoverKind)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn math_menu_button(label: &'static str, kind: MathPopoverKind) -> gtk::MenuButton {
    let mb = gtk::MenuButton::new();
    mb.set_label(label);
    mb.set_focusable(false);
    mb.add_css_class("function-button");
    MATH_BUTTONS.with(|list| {
        let weak = glib::WeakRef::new();
        weak.set(Some(&mb));
        list.borrow_mut().push((weak, kind));
    });
    mb
}

/// Invoke `f` for every registered x / f(x) menu button (the window attaches
/// the popover and refreshes its content on open).
pub fn for_each_math_button(f: impl Fn(&gtk::MenuButton, MathPopoverKind)) {
    MATH_BUTTONS.with(|list| {
        for (weak, kind) in list.borrow().iter() {
            if let Some(mb) = weak.upgrade() {
                f(&mb, *kind);
            }
        }
    });
}

// ---------------------------------------------------------------------
// Responsive two-page container (doc 13 §6.1)
// ---------------------------------------------------------------------

/// Two 5-column grids: side by side at ≥ 700 px, a swipeable carousel with
/// page dots below the breakpoint (doc 13 §6.1–6.2).
pub struct ResponsivePads {
    pub root: gtk::Box,
    grid_a: Grid,
    grid_b: Grid,
    wide: gtk::Box,
    car_box: gtk::Box,
    carousel: adw::Carousel,
    pages: [gtk::Box; 2],
    collapsed: Cell<bool>,
}

fn responsive(grid_a: Grid, grid_b: Grid) -> ResponsivePads {
    let wide = gtk::Box::new(Orientation::Horizontal, 4);
    // Fill: the two 336 px grids expand with the viewport above 700 px.
    wide.append(&grid_a);
    wide.append(&grid_b);

    let carousel = adw::Carousel::new();
    // No vexpand: an explicit vexpand here would leak through the keypad
    // stack (expand flags on hidden children still count) and stretch the
    // button rows on every page; the keypad pins to its minimum height.
    let page_a = gtk::Box::new(Orientation::Vertical, 0);
    let page_b = gtk::Box::new(Orientation::Vertical, 0);
    carousel.append(&page_a);
    carousel.append(&page_b);
    let dots = adw::CarouselIndicatorDots::new();
    dots.set_carousel(Some(&carousel));
    let car_box = gtk::Box::new(Orientation::Vertical, 6);
    car_box.append(&carousel);
    car_box.append(&dots);
    car_box.set_visible(false);

    let root = gtk::Box::new(Orientation::Vertical, 0);
    root.append(&wide);
    root.append(&car_box);

    ResponsivePads {
        root,
        grid_a,
        grid_b,
        wide,
        car_box,
        carousel,
        pages: [page_a, page_b],
        collapsed: Cell::new(false),
    }
}

impl ResponsivePads {
    /// Register an additional carousel page — programming's bit panel page
    /// (upstream MultiLayoutView third slot, D25). Advanced/financial keep
    /// two pages, so their dots stay at two.
    pub(crate) fn add_page(&self, w: &gtk::Widget) {
        self.carousel.append(w);
    }

    pub(crate) fn set_collapsed(&self, c: bool) {
        if self.collapsed.get() == c {
            return;
        }
        self.collapsed.set(c);
        if c {
            self.wide.remove(&self.grid_a);
            self.wide.remove(&self.grid_b);
            self.pages[0].append(&self.grid_a);
            self.pages[1].append(&self.grid_b);
        } else {
            self.pages[0].remove(&self.grid_a);
            self.pages[1].remove(&self.grid_b);
            self.wide.append(&self.grid_a);
            self.wide.append(&self.grid_b);
        }
        self.wide.set_visible(!c);
        self.car_box.set_visible(c);
    }
}

/// Smart brackets: `(` or `)` chosen by the window at click time.
fn smart_brackets(sink: &PadSink) -> Button {
    paren_button("( )", PadAction::SmartBrackets, sink)
}

fn ins(text: &str) -> PadAction {
    PadAction::Insert {
        text: text.to_string(),
        digit: false,
    }
}

/// `name(` insert for inverse-pair function buttons (label "sin" → "sin(").
fn ins_call(name: &str) -> PadAction {
    PadAction::Insert {
        text: format!("{name}("),
        digit: false,
    }
}

fn dig(d: &str) -> PadAction {
    PadAction::Insert {
        text: d.to_string(),
        digit: true,
    }
}

fn sup2() -> PadAction {
    PadAction::Insert {
        text: "²".to_string(),
        digit: false,
    }
}

fn ins10x() -> PadAction {
    PadAction::Insert {
        text: "×10^".to_string(),
        digit: false,
    }
}

// ---------------------------------------------------------------------
// 5.1 Basic + the shared numeric core (doc 13 §6 / reference layout)
// ---------------------------------------------------------------------

/// The numeric core grid shared by basic mode and the right-hand page of
/// advanced/financial (doc 13 §6): clear/shift row, digits, a tall `=`
/// spanning the bottom two rows, and the x / f(x) menus in the last row.
fn numeric_core(sink: &PadSink) -> Grid {
    let grid = math_grid();

    attach(&grid, &clear_button(PadAction::Clear, sink), 0, 0, 1, 1);
    attach(
        &grid,
        &mode_toggle("↑n", 0, PadAction::SupMode, sink),
        1,
        0,
        1,
        1,
    );
    attach(
        &grid,
        &mode_toggle("↓n", 1, PadAction::SubMode, sink),
        2,
        0,
        1,
        1,
    );
    attach(&grid, &op_button("÷", ins("÷"), sink), 3, 0, 1, 1);
    attach(&grid, &fn_button("mod", ins(" mod "), sink), 4, 0, 1, 1);

    let digits = [("7", "8", "9"), ("4", "5", "6"), ("1", "2", "3")];
    for (i, (a, bb, c)) in digits.iter().enumerate() {
        let row = i as i32 + 1;
        attach(&grid, &num_button(a, dig(a), sink), 0, row, 1, 1);
        attach(&grid, &num_button(bb, dig(bb), sink), 1, row, 1, 1);
        attach(&grid, &num_button(c, dig(c), sink), 2, row, 1, 1);
    }
    attach(&grid, &op_button("×", ins("×"), sink), 3, 1, 1, 1);
    attach(&grid, &pct_button(ins("%"), sink), 4, 1, 1, 1);
    attach(&grid, &op_button("−", ins("−"), sink), 3, 2, 1, 1);
    attach(&grid, &smart_brackets(sink), 4, 2, 1, 1);
    attach(&grid, &op_button("+", ins("+"), sink), 3, 3, 1, 1);
    attach(&grid, &eq_button(sink), 4, 3, 1, 2);
    attach(&grid, &num_button("0", dig("0"), sink), 0, 4, 1, 1);
    attach(&grid, &point_button(ins("."), sink), 1, 4, 1, 1);
    attach(
        &grid,
        &math_menu_button("x", MathPopoverKind::Variables),
        2,
        4,
        1,
        1,
    );
    attach(
        &grid,
        &math_menu_button("f(x)", MathPopoverKind::Functions),
        3,
        4,
        1,
        1,
    );

    grid
}

pub fn basic(sink: &PadSink) -> Grid {
    numeric_core(sink)
}

// ---------------------------------------------------------------------
// 5.2 Advanced (two 5-column pages, doc 13 §6)
// ---------------------------------------------------------------------

pub fn advanced(sink: &PadSink) -> ResponsivePads {
    let ga = math_grid();
    let inv = |n: (&'static str, PadAction), i: (&'static str, PadAction), css: &'static str| {
        inverse_pair_button(
            InversePair {
                normal: n,
                inverse: i,
            },
            css,
            sink,
        )
    };

    // Page A, row 0 — ⇧⁻¹ modifier toggle, power/root pairs, complex tools.
    attach(&ga, &inverse_mod_button(sink), 0, 0, 1, 1);
    attach(
        &ga,
        &inv(("x²", sup2()), ("y√", ins("ⁿ√")), "function-button"),
        1,
        0,
        1,
        1,
    );
    attach(
        &ga,
        &inv(("x^y", ins("^")), ("y×10ˣ", ins10x()), "function-button"),
        2,
        0,
        1,
        1,
    );
    attach(&ga, &fn_button("conj", ins("conj("), sink), 3, 0, 1, 1);
    attach(
        &ga,
        &inv(("x⁻¹", ins("⁻¹")), ("x¹", ins("¹")), "function-button"),
        4,
        0,
        1,
        1,
    );
    // Rows 1-3 — trig/log families with the complex helpers among them;
    // log/ln swap to 10ˣ/eˣ under ⇧⁻¹ (doc 06 §5.2).
    attach(
        &ga,
        &inv(
            ("sin", ins_call("sin")),
            ("sin⁻¹", ins_call("sin⁻¹")),
            "function-button",
        ),
        0,
        1,
        1,
        1,
    );
    attach(
        &ga,
        &inv(
            ("sinh", ins_call("sinh")),
            ("sinh⁻¹", ins_call("sinh⁻¹")),
            "function-button",
        ),
        1,
        1,
        1,
        1,
    );
    attach(
        &ga,
        &markup_button("log<sub>y</sub>", ins("log"), sink),
        2,
        1,
        1,
        1,
    );
    attach(
        &ga,
        &inv(
            ("Arg", ins("Arg(")),
            ("arg", ins("arg(")),
            "function-button",
        ),
        3,
        1,
        1,
        1,
    );
    attach(&ga, &fn_button("[x]", ins("["), sink), 4, 1, 1, 1);
    attach(
        &ga,
        &inv(
            ("cos", ins_call("cos")),
            ("cos⁻¹", ins_call("cos⁻¹")),
            "function-button",
        ),
        0,
        2,
        1,
        1,
    );
    attach(
        &ga,
        &inv(
            ("cosh", ins_call("cosh")),
            ("cosh⁻¹", ins_call("cosh⁻¹")),
            "function-button",
        ),
        1,
        2,
        1,
        1,
    );
    attach(
        &ga,
        &inv(
            ("log", ins("log(")),
            ("10^x", ins("10^")),
            "function-button",
        ),
        2,
        2,
        1,
        1,
    );
    attach(
        &ga,
        &inv(("Re", ins("Re(")), ("re", ins("Re(")), "function-button"),
        3,
        2,
        1,
        1,
    );
    attach(&ga, &fn_button("|x|", ins("|"), sink), 4, 2, 1, 1);
    attach(
        &ga,
        &inv(
            ("tan", ins_call("tan")),
            ("tan⁻¹", ins_call("tan⁻¹")),
            "function-button",
        ),
        0,
        3,
        1,
        1,
    );
    attach(
        &ga,
        &inv(
            ("tanh", ins_call("tanh")),
            ("tanh⁻¹", ins_call("tanh⁻¹")),
            "function-button",
        ),
        1,
        3,
        1,
        1,
    );
    attach(
        &ga,
        &inv(("ln", ins("ln(")), ("e^x", ins("e^")), "function-button"),
        2,
        3,
        1,
        1,
    );
    attach(
        &ga,
        &inv(("Im", ins("Im(")), ("im", ins("Im(")), "function-button"),
        3,
        3,
        1,
        1,
    );
    attach(&ga, &fn_button("x!", ins("!"), sink), 4, 3, 1, 1);
    // Row 4 — angle unit, constants, factorize.
    attach(
        &ga,
        &fn_button("Deg", PadAction::AngleCycle, sink),
        0,
        4,
        1,
        1,
    );
    attach(&ga, &fn_button("π", ins("π"), sink), 1, 4, 1, 1);
    attach(&ga, &fn_button("e", ins("e"), sink), 2, 4, 1, 1);
    attach(&ga, &fn_button("i", ins("i"), sink), 3, 4, 1, 1);
    attach(
        &ga,
        &fn_button("a×b", PadAction::Factorize, sink),
        4,
        4,
        1,
        1,
    );

    // Page B — the shared numeric core.
    let gb = numeric_core(sink);

    responsive(ga, gb)
}

// ---------------------------------------------------------------------
// 5.3 Programming (doc 13 §6.4: bit panel attaches under the display card)
// ---------------------------------------------------------------------

pub struct ProgrammingPads {
    pub root: gtk::Box,
    /// Bit toggles panel — lives inside the display card (doc 13 §6.4), so
    /// it follows the card's small-mode margins (D24).
    pub bit_panel: gtk::Box,
    /// Bit toggles, bit 63 first — the wide arrangement: two 32-bit rows
    /// with 63/47/32 and 31/15/0 markers at the row ends (upstream 51.0
    /// bit panel, D25).
    pub bits: Vec<ToggleButton>,
    /// Narrow (carousel breakpoint) arrangement: four 16-bit rows with the
    /// range markers under each row.
    pub narrow_bits: Vec<ToggleButton>,
    wide_bits_grid: Grid,
    narrow_bits_grid: Grid,
    /// Carousel page hosting the bit panel below the breakpoint (upstream
    /// MultiLayoutView narrow layout, D25) — empty while wide, when the
    /// panel lives at the end of the display card.
    bit_page: gtk::Box,
    /// Which parent currently holds the bit panel — remove/append are
    /// asserted in GTK, so a repeated set_collapsed must no-op.
    bit_collapsed: Cell<bool>,
    /// Hex digit buttons with their numeric values (greyed out when the
    /// value is not representable in the current base, D25).
    hex_buttons: Vec<(u32, Button)>,
    /// Base/word-size menu buttons (in the keypad's bottom row, upstream).
    pub base_btn: gtk::MenuButton,
    pub word_btn: gtk::MenuButton,
    pub resp: ResponsivePads,
}

impl ProgrammingPads {
    /// Grey out hex digits the current base cannot represent (upstream
    /// update_hex_button_sensitivities).
    pub fn set_base_sensitivity(&self, base: u32) {
        for (value, b) in &self.hex_buttons {
            b.set_sensitive(*value < base);
        }
    }

    /// Wide ↔ carousel for the two grids (doc 13 §6.1). Below the
    /// breakpoint the bit panel leaves the display card and becomes the
    /// carousel's third page, matching the upstream narrow layout; wide,
    /// it re-attaches at the card's end (history, entry, status, bits).
    pub(crate) fn set_collapsed(&self, c: bool, card: &gtk::Box) {
        self.resp.set_collapsed(c);
        self.wide_bits_grid.set_visible(!c);
        self.narrow_bits_grid.set_visible(c);
        if self.bit_collapsed.get() == c {
            return;
        }
        self.bit_collapsed.set(c);
        if c {
            card.remove(&self.bit_panel);
            self.bit_page.append(&self.bit_panel);
            self.bit_panel.add_css_class("in-carousel");
        } else {
            self.bit_page.remove(&self.bit_panel);
            card.append(&self.bit_panel);
            self.bit_panel.remove_css_class("in-carousel");
        }
    }
}

/// Flat bit toggle: tnum chip labeled 0/1, tooltip = bit index; a group
/// separator after every 4-bit run (upstream bit-toggle-separator).
fn bit_button(bit: u32, sink: &PadSink) -> ToggleButton {
    let bt = ToggleButton::new();
    bt.set_focusable(false);
    bt.add_css_class("flat");
    bt.add_css_class("bit-toggle-button");
    bt.set_tooltip_text(Some(&bit.to_string()));
    if bit.is_multiple_of(4) && !bit.is_multiple_of(16) {
        bt.add_css_class("bit-toggle-separator");
    }
    let lbl = Label::new(Some("0"));
    bt.set_child(Some(&lbl));
    let sink = Rc::clone(sink);
    bt.connect_clicked(move |_| sink(PadAction::Bit(bit)));
    bt
}

fn bit_marker(text: &str) -> Label {
    let m = Label::new(Some(text));
    m.add_css_class("bit-marker-label");
    m
}

pub fn programming(sink: &PadSink) -> ProgrammingPads {
    let root = gtk::Box::new(Orientation::Vertical, 0);
    let inv = |n: (&'static str, PadAction), i: (&'static str, PadAction), css: &'static str| {
        inverse_pair_button(
            InversePair {
                normal: n,
                inverse: i,
            },
            css,
            sink,
        )
    };

    // Bit panel (upstream 51.0 arrangement, D25): wide = two 32-bit rows —
    // "63 | bits 63–48 | 47 | bits 47–32 | 32" and the 31…0 row below —
    // narrow = four 16-bit rows with range markers under each. Both live
    // in one panel and swap on the carousel breakpoint.
    let bit_panel = gtk::Box::new(Orientation::Vertical, 0);
    bit_panel.add_css_class("bit-panel");

    let wide_bits_grid = Grid::new();
    wide_bits_grid.set_row_spacing(4);
    wide_bits_grid.set_column_spacing(2);
    wide_bits_grid.set_hexpand(true);
    let mut bits = Vec::new();
    for (row, start, left, mid, right) in
        [(0u32, 63u32, "63", "47", "32"), (2, 31, "31", "15", "0")]
    {
        let m = bit_marker(left);
        m.set_halign(gtk::Align::Start);
        attach(&wide_bits_grid, &m, 0, row as i32, 1, 1);
        for i in 0..16u32 {
            let b = bit_button(start - i, sink);
            b.set_hexpand(true);
            attach(&wide_bits_grid, &b, 1 + i as i32, row as i32, 1, 1);
            bits.push(b);
        }
        let m = bit_marker(mid);
        m.set_halign(gtk::Align::Center);
        attach(&wide_bits_grid, &m, 17, row as i32, 1, 1);
        for i in 0..16u32 {
            let b = bit_button(start - 16 - i, sink);
            b.set_hexpand(true);
            attach(&wide_bits_grid, &b, 18 + i as i32, row as i32, 1, 1);
            bits.push(b);
        }
        let m = bit_marker(right);
        m.set_halign(gtk::Align::End);
        attach(&wide_bits_grid, &m, 34, row as i32, 1, 1);
    }
    bit_panel.append(&wide_bits_grid);

    // Narrow arrangement (carousel breakpoint, D25): four 16-bit rows with
    // the range markers under each — the upstream carousel bit-panel page.
    // 16 tnum chips fit a 336 px carousel page where a full 64-bit row
    // (≈ 900 px minimum) can never; the four-row stack rides the carousel
    // page instead of the display card, so it costs no window height.
    let narrow_bits_grid = Grid::new();
    narrow_bits_grid.set_row_spacing(2);
    narrow_bits_grid.set_column_spacing(2);
    narrow_bits_grid.set_visible(false);
    let mut narrow_bits = Vec::new();
    for (row, start) in [(0u32, 63u32), (2, 47), (4, 31), (6, 15)] {
        for i in 0..16u32 {
            let b = bit_button(start - i, sink);
            b.set_hexpand(true);
            attach(&narrow_bits_grid, &b, i as i32, row as i32, 1, 1);
            narrow_bits.push(b);
        }
        let m = bit_marker(&start.to_string());
        m.set_halign(gtk::Align::Start);
        attach(&narrow_bits_grid, &m, 0, row as i32 + 1, 1, 1);
        let m = bit_marker(&(start - 15).to_string());
        m.set_halign(gtk::Align::End);
        attach(&narrow_bits_grid, &m, 15, row as i32 + 1, 1, 1);
    }
    bit_panel.append(&narrow_bits_grid);

    // Base + word size live in the keypad's bottom row as up-opening menu
    // buttons (upstream calc_base_button / calc_word_size_button, D25).
    let base_menu = gio::Menu::new();
    for (name, base) in [
        ("Binary", 2u32),
        ("Octal", 8),
        ("Decimal", 10),
        ("Hexadecimal", 16),
    ] {
        let it = gio::MenuItem::new(Some(name), None);
        it.set_action_and_target_value(Some("win.prog-base"), Some(&base.to_string().to_variant()));
        base_menu.append_item(&it);
    }
    let base_btn = gtk::MenuButton::new();
    base_btn.set_label("Decimal");
    base_btn.set_menu_model(Some(&base_menu));
    base_btn.set_direction(gtk::ArrowType::Up);
    base_btn.set_can_shrink(true);
    base_btn.set_focusable(false);
    base_btn.add_css_class("function-button");
    base_btn.set_tooltip_text(Some("Change Base"));

    let word_menu = gio::Menu::new();
    for word in [8u32, 16, 32, 64] {
        let it = gio::MenuItem::new(Some(&format!("{word}-bit")), None);
        it.set_action_and_target_value(Some("win.prog-word"), Some(&word.to_string().to_variant()));
        word_menu.append_item(&it);
    }
    let word_btn = gtk::MenuButton::new();
    word_btn.set_label("64-bit");
    word_btn.set_menu_model(Some(&word_menu));
    word_btn.set_direction(gtk::ArrowType::Up);
    word_btn.set_can_shrink(true);
    word_btn.set_focusable(false);
    word_btn.add_css_class("function-button");
    word_btn.set_tooltip_text(Some("Change Word Size"));

    // Two 5-column grids (doc 13 §6.1): upstream 51.0 layout — advanced
    // functions left, basic digits right, hex digits in a nested five-
    // column grid inside the three leftmost basic columns, operators in
    // the two rightmost, tall `=` spanning the last two rows (D25).
    let ga = math_grid();
    attach(&ga, &inverse_mod_button(sink), 0, 0, 1, 1);
    attach(
        &ga,
        &inv(("x²", sup2()), ("√", ins("√")), "function-button"),
        1,
        0,
        1,
        1,
    );
    attach(
        &ga,
        &inv(("xⁿ", ins("^")), ("ⁿ√", ins("ⁿ√")), "function-button"),
        2,
        0,
        1,
        1,
    );
    attach(&ga, &fn_button("twos", ins("twos("), sink), 3, 0, 1, 1);
    attach(&ga, &fn_button("⌈x⌉", ins("⌈"), sink), 4, 0, 1, 1);
    attach(
        &ga,
        &inv(("∧", ins(" ∧ ")), ("⊼", ins(" ⊼ ")), "function-button"),
        0,
        1,
        1,
        1,
    );
    attach(
        &ga,
        &inv(("∨", ins(" ∨ ")), ("⊽", ins(" ⊽ ")), "function-button"),
        1,
        1,
        1,
        1,
    );
    attach(
        &ga,
        &inv(
            ("log<sub>y</sub>", ins("log")),
            ("<i>y</i><sup>x</sup>", ins("^")),
            "function-button",
        ),
        2,
        1,
        1,
        1,
    );
    attach(&ga, &fn_button("swap", ins("swap("), sink), 3, 1, 1, 1);
    attach(&ga, &fn_button("⌊x⌋", ins("⌊"), sink), 4, 1, 1, 1);
    attach(&ga, &fn_button("¬", ins("¬"), sink), 0, 2, 1, 1);
    attach(
        &ga,
        &inv(("⊻", ins(" ⊻ ")), ("⊙", ins(" ⊙ ")), "function-button"),
        1,
        2,
        1,
        1,
    );
    attach(
        &ga,
        &inv(
            ("log", ins("log(")),
            ("10<sup>x</sup>", ins("10^")),
            "function-button",
        ),
        2,
        2,
        1,
        1,
    );
    attach(&ga, &fn_button("int", ins("int("), sink), 3, 2, 1, 1);
    attach(&ga, &fn_button("|x|", ins("|"), sink), 4, 2, 1, 1);
    attach(&ga, &fn_button("≪", ins(" ≪ "), sink), 0, 3, 1, 1);
    attach(
        &ga,
        &inv(("≫", ins(" ≫ ")), ("⋙", ins(" ⋙ ")), "function-button"),
        1,
        3,
        1,
        1,
    );
    attach(
        &ga,
        &inv(
            ("log₂", ins("log₂(")),
            ("2<sup>x</sup>", ins("2^")),
            "function-button",
        ),
        2,
        3,
        1,
        1,
    );
    attach(&ga, &fn_button("frac", ins("frac("), sink), 3, 3, 1, 1);
    attach(&ga, &fn_button("x!", ins("!"), sink), 4, 3, 1, 1);
    attach(&ga, &base_btn, 0, 4, 2, 1);
    attach(&ga, &word_btn, 2, 4, 2, 1);
    attach(
        &ga,
        &fn_button("á", PadAction::CharDialog, sink),
        4,
        4,
        1,
        1,
    );

    let gb = math_grid();
    attach(&gb, &clear_button(PadAction::Clear, sink), 0, 0, 1, 1);
    attach(
        &gb,
        &mode_toggle("↑n", 0, PadAction::SupMode, sink),
        1,
        0,
        1,
        1,
    );
    attach(
        &gb,
        &mode_toggle("↓n", 1, PadAction::SubMode, sink),
        2,
        0,
        1,
        1,
    );
    attach(&gb, &op_button("÷", ins("÷"), sink), 3, 0, 1, 1);
    attach(&gb, &fn_button("mod", ins(" mod "), sink), 4, 0, 1, 1);

    // Nested digit grid (upstream): five narrow columns inside the three
    // leftmost basic columns — B–F over 6–9,A over 1–5; digits the current
    // base cannot represent grey out (set_base_sensitivity).
    let hex_grid = Grid::new();
    hex_grid.set_row_spacing(4);
    hex_grid.set_column_spacing(4);
    hex_grid.set_row_homogeneous(true);
    hex_grid.set_column_homogeneous(true);
    hex_grid.set_hexpand(true);
    hex_grid.set_vexpand(true);
    let mut hex_buttons = Vec::new();
    for (row, labels, start_value) in [
        (0u32, ["B", "C", "D", "E", "F"], 11u32),
        (1, ["6", "7", "8", "9", "A"], 6),
        (2, ["1", "2", "3", "4", "5"], 1),
    ] {
        for (c, l) in labels.iter().enumerate() {
            let b = num_button(l, dig(l), sink);
            hex_buttons.push((start_value + c as u32, b.clone()));
            attach(&hex_grid, &b, c as i32, row as i32, 1, 1);
        }
    }
    attach(&gb, &hex_grid, 0, 1, 3, 3);
    attach(&gb, &op_button("×", ins("×"), sink), 3, 1, 1, 1);
    attach(&gb, &pct_button(ins("%"), sink), 4, 1, 1, 1);
    attach(&gb, &op_button("−", ins("−"), sink), 3, 2, 1, 1);
    attach(&gb, &smart_brackets(sink), 4, 2, 1, 1);
    attach(&gb, &op_button("+", ins("+"), sink), 3, 3, 1, 1);
    attach(&gb, &eq_button(sink), 4, 3, 1, 2);
    attach(&gb, &num_button("0", dig("0"), sink), 0, 4, 1, 1);
    attach(&gb, &point_button(ins("."), sink), 1, 4, 1, 1);
    attach(
        &gb,
        &math_menu_button("x", MathPopoverKind::Variables),
        2,
        4,
        1,
        1,
    );
    attach(
        &gb,
        &math_menu_button("f(x)", MathPopoverKind::Functions),
        3,
        4,
        1,
        1,
    );

    // Third carousel page (D25): below the breakpoint the bit panel is
    // swiped to like the upstream MultiLayoutView bit_panel slot, which
    // sidesteps its 64-bit minimum width inside a 360 px card.
    let resp = responsive(ga, gb);
    let bit_page = gtk::Box::new(Orientation::Vertical, 0);
    resp.add_page(bit_page.upcast_ref());
    root.append(&resp.root);

    ProgrammingPads {
        root,
        bit_panel,
        bits,
        narrow_bits,
        wide_bits_grid,
        narrow_bits_grid,
        bit_page,
        bit_collapsed: Cell::new(false),
        hex_buttons,
        base_btn,
        word_btn,
        resp,
    }
}

// ---------------------------------------------------------------------
// 5.4 Financial (two 5-column pages, doc 13 §6)
// ---------------------------------------------------------------------

pub fn financial(sink: &PadSink) -> ResponsivePads {
    let ga = math_grid();
    let inv = |n: (&'static str, PadAction), i: (&'static str, PadAction), css: &'static str| {
        inverse_pair_button(
            InversePair {
                normal: n,
                inverse: i,
            },
            css,
            sink,
        )
    };

    // Page A, row 0 — the same math-function row as advanced.
    attach(&ga, &inverse_mod_button(sink), 0, 0, 1, 1);
    attach(
        &ga,
        &inv(("x²", sup2()), ("y√", ins("ⁿ√")), "function-button"),
        1,
        0,
        1,
        1,
    );
    attach(
        &ga,
        &inv(("x^y", ins("^")), ("y×10ˣ", ins10x()), "function-button"),
        2,
        0,
        1,
        1,
    );
    attach(
        &ga,
        &inv(
            ("log", ins("log(")),
            ("10^x", ins("10^")),
            "function-button",
        ),
        3,
        0,
        1,
        1,
    );
    attach(
        &ga,
        &inv(("ln", ins("ln(")), ("e^x", ins("e^")), "function-button"),
        4,
        0,
        1,
        1,
    );

    // Rows 1-4 — the ten dialog formulas (doc 04 §4) as wide thirds, then
    // Term (third) + Currency Conversion (two thirds). Each row is a nested
    // grid spanning the five 64 px columns so the buttons read wider.
    let fd = |name: &'static str| PadAction::FinancialDialog(name);
    let rows: [[(&'static str, PadAction); 3]; 3] = [
        [("Ctrm", fd("Ctrm")), ("Ddb", fd("Ddb")), ("Fv", fd("Fv"))],
        [("Gpm", fd("Gpm")), ("Pmt", fd("Pmt")), ("Pv", fd("Pv"))],
        [("Rate", fd("Rate")), ("Sln", fd("Sln")), ("Syd", fd("Syd"))],
    ];
    for (r, trio) in rows.iter().enumerate() {
        let row_grid = Grid::new();
        row_grid.set_column_spacing(4);
        row_grid.set_hexpand(true);
        for (c, (label, action)) in trio.iter().enumerate() {
            let b = fn_button(label, action.clone(), sink);
            b.set_hexpand(true);
            row_grid.attach(&b, c as i32, 0, 1, 1);
        }
        attach(&ga, &row_grid, 0, r as i32 + 1, 5, 1);
    }
    let last_row = Grid::new();
    last_row.set_column_spacing(4);
    last_row.set_hexpand(true);
    let term = fn_button("Term", fd("Term"), sink);
    term.set_hexpand(true);
    last_row.attach(&term, 0, 0, 1, 1);
    let conv = fn_button("Currency Conversion", PadAction::ConversionView, sink);
    conv.set_hexpand(true);
    last_row.attach(&conv, 1, 0, 2, 1);
    attach(&ga, &last_row, 0, 4, 5, 1);

    // Page B — the shared numeric core.
    let gb = numeric_core(sink);

    responsive(ga, gb)
}

// ---------------------------------------------------------------------
// 5.5 Conversion keypad (live conversion, no equals)
// ---------------------------------------------------------------------

/// Conversion keypad (doc 13 §6 reference): three columns — C/⇅/backspace
/// over the digits, with the category-contextual button in the last cell
/// (°′″ for angle, rate details for currency, π elsewhere; doc 06 §5.5).
/// Hex digits A-F swap in over the decimal digits for Number Base.
pub struct ConversionPad {
    pub grid: Grid,
    /// The decimal-digit grid, overlaid with `hex_buttons` in one cell and
    /// hidden while the hex block shows (upstream overlay: a hidden grid
    /// ROW still counts toward the grid minimum, so the blocks must swap
    /// inside a single cell instead of occupying extra rows).
    pub digits_grid: Grid,
    /// Hex-digit buttons (hidden unless the Number Base category is shown).
    pub hex_buttons: Vec<Button>,
    /// Category-contextual buttons, all attached to the last cell; exactly
    /// one is visible at a time (converter picks per category).
    pub dms_btn: Button,
    pub rate_btn: Button,
    pub pi_btn: Button,
}

pub fn conversion(sink: &PadSink) -> ConversionPad {
    let grid = math_grid();

    let clear = clear_button(PadAction::Clear, sink);
    attach(&grid, &clear, 0, 0, 1, 1);
    attach(
        &grid,
        &fn_button("⇅", PadAction::SwapUnits, sink),
        1,
        0,
        1,
        1,
    );
    let back = Button::from_icon_name("edit-clear-symbolic");
    connect_action(&back, PadAction::Backspace, sink);
    back.add_css_class("function-button");
    attach(&grid, &back, 2, 0, 1, 1);

    // Digit cell (rows 1-3): the decimal and hex blocks are two nested
    // 3-column grids overlaid in the same cell (upstream buttons-conversion).
    // Attaching hex as hidden rows of the outer grid instead would add two
    // phantom rows to the grid minimum (~343 px instead of 236), clipping
    // the keypad in the 616 px window.
    let digit_cell = |title: &str| {
        let g = Grid::new();
        g.set_row_spacing(4);
        g.set_column_spacing(4);
        g.set_row_homogeneous(true);
        g.set_column_homogeneous(true);
        g.set_hexpand(true);
        g.set_vexpand(true);
        g.set_visible(title == "dec");
        g
    };
    let dec = digit_cell("dec");
    let digits = [("7", "8", "9"), ("4", "5", "6"), ("1", "2", "3")];
    for (i, (a, bb, c)) in digits.iter().enumerate() {
        let row = i as i32;
        attach(&dec, &num_button(a, dig(a), sink), 0, row, 1, 1);
        attach(&dec, &num_button(bb, dig(bb), sink), 1, row, 1, 1);
        attach(&dec, &num_button(c, dig(c), sink), 2, row, 1, 1);
    }
    attach(&grid, &dec, 0, 1, 3, 3);

    let hex_grid = digit_cell("hex");
    let mut hex_buttons = Vec::new();
    let hex = ["A", "B", "C", "D", "E", "F"];
    for (i, h) in hex.iter().enumerate() {
        let bt = num_button(h, dig(h), sink);
        bt.set_visible(false);
        attach(&hex_grid, &bt, i as i32 % 3, i as i32 / 3, 1, 1);
        hex_buttons.push(bt);
    }
    attach(&grid, &hex_grid, 0, 1, 3, 3);

    attach(&grid, &num_button("0", dig("0"), sink), 0, 4, 1, 1);
    attach(&grid, &point_button(ins("."), sink), 1, 4, 1, 1);

    // Contextual last cell (doc 06 §5.5): one button per category family.
    let dms_btn = fn_button("° ′ ″", PadAction::Dms, sink);
    let rate_btn = fn_button("Rate", PadAction::RateDetails, sink);
    let pi_btn = fn_button("π", ins("π"), sink);
    for b in [&dms_btn, &rate_btn, &pi_btn] {
        b.set_visible(false);
        attach(&grid, b, 2, 4, 1, 1);
    }

    ConversionPad {
        grid,
        digits_grid: dec,
        hex_buttons,
        dms_btn,
        rate_btn,
        pi_btn,
    }
}
