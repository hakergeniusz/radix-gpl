# 06 — UI and Interaction Specification

**Scope:** every user-visible screen, panel, dialog, keypad, and interaction, toolkit-independently. Widget names appear only where they identify a user-visible element. Visual styling (geometry, colors, typography, per-state looks, iconography) is specified in `13`.
**Evidence:** direct source analysis of the application shell, window, keypad, and dialog logic, with all keypad layouts extracted programmatically from the UI definition files; settings schema and help pages. Confidence: **V** / **S** / **I** as in `02`.

---

## 1. Startup [V]

1. Application launches (multiple instances allowed — `NON_UNIQUE`; each instance is independent).
2. Settings read: accuracy, word-size, base, show-thousands, show-zeroes, number-format, angle-units, button-mode, unit-category, source-units, target-units, window-maximized, window-size, refresh-interval, favorite-currencies, enabled-completions, history-tape.
3. One window created: default size from `window-size` (unless maximized), title "Calculator", icon `org.gnome.Calculator`; keypad mode from `button-mode`; converter category/units restored; internal precision set (2000 default); currency refresh started in background.
4. History tape restored asynchronously shortly after showing.
5. Keyboard-mode/conversion-mode restore like any other mode.
6. With `--equation <text>`: each non-empty line of the (escape-unescaped, `\n`-split) text is placed into the display; empty lines trigger solve of the current display.
7. With `--mode <m>`: mode switched after showing.

## 2. Window layout [V]

```
┌ Header bar ────────────────────────────────────────────────┐
│ [Mode menu ▾]   [undo] [redo]        (title)  [≡ main menu] │
├ History view (scrollable list, shown when it has entries) ──┤
├ Display (entry area + status/info line + optional spinner) ┤
├ Converter panel (when applicable) ──────────────────────────┤
├ Keypad panel (per mode) ────────────────────────────────────┤
└─────────────────────────────────────────────────────────────┘
```

- Header bar: mode menu button (labeled with current mode name: Basic/Advanced/Financial/Programming/Keyboard/Conversion), undo button, redo button (hidden in conversion mode), main menu button ("Primary menu").
- In programming mode the display container gains a "programming" style (rounded top); the info line shows the base label (see §5.3).
- Minimum height request 616 px; default widths per mode: Basic/Conversion 360 sp, others 700 sp.
- Development builds add a "(Nightly)" prefix to the window name and a devel style class.

## 3. Menus [V]

**Mode menu (header-left):** Basic, Advanced, Financial, Programming, Keyboard, Conversion (accelerators Ctrl+Alt+B/A/F/P/K/T/C respectively).

**Primary menu (header-right):**
- New Window (Ctrl+N)
- Clear History (win.clear; Ctrl+Escape)
- Result Format section: Automatic / Fixed / Scientific / Engineering (radio items; persisted as `number-format`)
- Preferences (Ctrl+comma)
- Keyboard Shortcuts (Ctrl+question)
- Help (F1)
- About Calculator

**About dialog:** name, version, license GPL-3.0, developers, translator credits, "Exchange Rate Data Providers" legal section listing the providers with links.

**Quit** (Ctrl+Q): with more than one window open, a confirmation dialog "Close All?" ("Are you sure you want to close all open windows?", Cancel/Close All with destructive styling) appears.

## 4. Display area [V]

- Multi-line editable text field (LTR direction always), monospace-ish styling with tabular numbers; horizontal scrolling; vertical scroll wheel scrolls horizontally (dy → horizontal step).
- Right margin/backspace button: a backspace button is shown next to the display in keypad modes (hidden in keyboard mode).
- Status/info line under the display: shows status messages (errors per `05` §4, "Calculating", "No sane value to store" etc.); in programming mode shows the **base label** when no error is active. Error state adds an "error" style class and announces the message to screen readers.
- Spinner during long solves (visible from 100 ms).
- Cursor scrolling: cursor kept visible with margins while moving.

## 5. Keypad panels

All keypads force LTR. Buttons never take focus on click. Digit buttons insert via the digit-insertion action (which respects superscript/subscript modes and disables completion while inserting). Numeric point button label is the locale radix character. The "x" button opens the variables/constants popover; "f(x)" opens the functions popover.

### 5.1 Basic [V] (24 buttons, grid)

Row 0: `C` (clear display), `↑n` (superscript mode toggle), `↓n` (subscript mode toggle), `÷`, `mod`
Rows 1-4: digits 789/456/123/0, `×`, `%`, `−`, `( )` (smart brackets), `+`, `=` (spans 2 rows), `.` (radix), `x` (variables), `f(x)` (functions).

### 5.2 Advanced [V] (60 buttons)

Carousel with two pages (users can swipe between a basic-like page and the advanced page; on mode entry the advanced page is shown). Advanced page layout (rows):

- Row 0: `C`, `⇧⁻¹` (inverse modifier toggle, bound to Shift key), `↑n`, `√` (or `ⁿ√` when inverse active), `↓n`, `÷`/`conj`, `mod`
- Row 1: 7 + `sin`/`sin⁻¹` (inverse-dependent), 8 + `sinh`/`sinh⁻¹`, 9, `×`, `Arg`, `%`
- Row 2: 4 + `cos`/`cos⁻¹`, 5 + `cosh`/`cosh⁻¹`, 6 + `log`/`ln`(inverse-dependent), `−`, `Re`, `( )`
- Row 3: 1 + `tan`/`tan⁻¹`, 2 + `tanh`/`tanh⁻¹`, 3, `+`, `Im`, `=` (spans 2)
- Row 4: 0, angle-unit button (label Rad/Deg/Grad; cycles Deg→Rad→Grad), `.`/`π`, `x`/`e`, `f(x)`/`i`, `a×b` (factorize)
- Inverse-modifier-dependent pair (shown in a lower row group; toggled by ⇧⁻¹ or holding Shift): `x²`/`y√` (nth root), `x^y`/`y×10^x` (exponent buttons: x^y inserts `^`; `10^x` and `e^x` insert exponent prefixes), `log_y` (logarithm subscript entry)
- Also: `x⁻¹` (inverse), `[x]` (round), `|x|` (absolute), `x!` (factorial)

Exact behavior of the inverse modifier: holding Shift or toggling ⇧⁻¹ swaps trig functions to their inverses and root/power buttons to their alternates (verified via visibility bindings).

### 5.3 Programming [V] (127 buttons)

- **Bit panel**: 64 bit-toggle buttons (bit 63…0), grouped with a visual separator between bit 32 and bit 31 (byte/word boundary styling). Clicking toggles the bit of the current value (two's complement for integers; IEEE-754 bit manipulation for floats at 32/64-bit word size, with exponent bits highlighted: bits 52–62 accent and 63 dimmed for double; 23–30 accent and 31 dimmed for float). Buttons beyond the word size are disabled. Bit panel is insensitive for complex values, empty display, or sign/radix-only display.
- **Base label** (in info line): the current value shown in the bases *other than the current one* with subscript suffixes and an LTR mark — in DEC mode: `octal₈ hex₁₆`; in BIN mode additionally `decimal₁₀`; negative values shown with `−` and the magnitude of the two's complement.
- **Base selector** menu button (label Binary/Octal/Decimal/Hexadecimal; also Ctrl+B/O/D/H): sets the parser base; digit buttons A–F disabled below base 16 (button sensitivity by base); the `10^x` exponent button's target changes with base (inserts `A` in hex, `12` in octal, `1010` in binary, `10` in decimal — the literal "10" rendered in the current base); a `2^x` button exists (target `2`).
- **Word size** menu button (label "8-bit"…"64-bit"): sets word size (also in Preferences).
- Buttons: `twos` (two's complement function), `swap` (bswap function), `int`, `frac`, `log`/`ln`(inverse), `log₂`, bitwise buttons `∧`/`⊼`, `∨`/`⊽`, `⊻`/`⊙` (XOR/XNOR inverse-dependent), `¬` (NOT), shifts `≪` `≫`/`⋙` (unsigned right shift on inverse), `mod`, `%`, `√`(inverse → nth root), `x²`/`x^y`(inverse), `log_y`, `10^x`, `2^x`, `⌈x⌉`, `⌊x⌋`, `|x|`, `x!`, `( )`, digits 0-9 A-F, `.`, `C`, `↑n`/`↓n`, `x`, `f(x)`, `=`.
- **Character dialog** (`á` button): converts between a character and its code; three modes selected by combo: (1) character → Unicode code point as number; (2) text → UTF-8 bytes as a big number (bytes shifted 8 bits each); (3) character → UTF-16 code units (surrogate pair encoded as a 32-bit number). Insert places the number into the display.
- Programming mode has no converter panel; number-base conversion is done via the base selector and `in` conversions.

### 5.4 Financial [V] (64 buttons)

Basic arithmetic grid (same as basic) plus financial dialog buttons in the left columns: Ctrm, Ddb, Fv, Gpm, Pmt, Pv, Rate, Sln, Syd, Term (each opens its dialog, see `04` §4), `Currency Conversion` (switches to conversion view with currency category), and inverse-dependent `√`/`log`/`ln` plus `x²`/`x^y`, `y√`, `10^x`, `e^x` exponent buttons. Carousel like advanced mode.

### 5.5 Conversion mode [V] (dedicated conversion view)

- Hides the expression display, undo/redo, copy and clear actions; shows only the converter panel and a numeric keypad.
- Keypad: digits, hex digits A-F (shown only for the Number Base category; disabled above the selected base), `.` radix, `⇅` swap units, backspace, `C` (clear), `° ′ ″` DMS insert button (angle category), `π`, `+/−` change sign, rate-details button (currency category), `=`... (no equals; conversions are live).
- Back button in header (labeled "Financial") returns to financial mode (also how one entered conversion mode from financial).

### 5.6 Keyboard mode [V]

Hides the keypad entirely; enables the on-screen keyboard for the display (OSK inhibit flag cleared); Ctrl+Alt+K or Ctrl+Alt+T to enter/leave; mode menu still shows "Keyboard".

## 6. Converter panel (conversion mode only) [V]

The converter is a window-level panel between display and keypad. It is visible **only in conversion mode**; in other modes conversions are done via `in` expressions in the display, or by switching to conversion mode (the financial keypad's "Currency Conversion" button does this with the Currency category preselected; help pages state conversions "must be performed using the keyboard or using the conversion mode").

- Category dropdown (all categories sorted by display name; Currency included; Number Base included).
- From/To unit dropdowns (searchable when > 10 units). Currency lists show favorites first (star icon; check mark on selected), sorted by code; display style per `currency-display` setting: code / name / both ("CODE (Name)").
- Two value fields (from, to), fully synced: editing either updates the other live; invalid input shows "Error" in the other field (and 0 is used for computation); focused field shows plain fixed-format value (unformatted for editing); unfocused shows user-formatted (accuracy/grouping) value; DMS unit formats as `d°m′s″`.
- π entry support: a field value may be `π`, `−π`, or `kπ` (multiplier×π) [V].
- Number Base category: fields render in the selected base; hex digit buttons enabled accordingly.
- Context menu on fields: Copy (strips thousands separator), Paste (replaces content; newlines→spaces).
- Currency status stack: while rates are not loaded, an "Exchange Rates Not Loaded" message with a "Refresh rates" link is shown instead of the fields; the link forces a refresh.
- Category/unit selections persisted at shutdown (`unit-category`, `source-units`, `target-units` — per-category last-used units are remembered in-session; only current category's pair persisted).
- Swap button exchanges from/to units and recomputes.

## 7. Variables and functions popovers [V]

- **Variables popover** (x button): lists user variables (name = value), plus `_` (previous answer, updates live) and `rand`. Rows: insert variable name into display at cursor (with spacing); delete (not for reserved); constants in category submenus (Mathematics, Electromagnetism, …) insert the constant symbol. Name entry at top with "store value" button: stores the currently displayed value (or pending solve result) under the entered name; validation live with error messages (see `04` §1.7 rules; name spaces are auto-replaced with underscores; invalid names disable the button and mark the field with an a11y invalid state).
- **Functions popover** (f(x) button): built-in functions grouped by category submenus; custom functions listed with edit (loads definition into display) and delete; "add" flow: enter name, choose argument count (1-10 spinner, named x,y,z,u,v,w,a,b,c,d), button inserts `name(x)=` into the display for the user to complete; validation as in `04`.

## 8. Preferences dialog [V]

(Adw preferences dialog, presented over the window; Ctrl+comma.)

| Control | Setting | Values |
|---|---|---|
| Number of decimals (spin) | accuracy | default 9 |
| Trailing zeroes (switch) | show-zeroes | off default |
| Thousands separators (switch) | show-thousands | off default |
| Angle units (combo) | angle-units | Degrees/Radians/Gradians |
| Word size (combo) | word-size | 8/16/32/64-bit |
| Currency refresh interval (combo) | refresh-interval | Never/Daily/Weekly |
| Currency display (combo) | currency-display | Currency Code/Name/Both |
| Currency completion (combo) | enabled-completions | All currencies / Favorites only / None |
| Favorite currencies (subpage) | favorite-currencies | searchable list of switches per currency |

Number format is NOT in preferences; it is in the primary menu. All changes apply immediately (bindings live).

## 9. Keyboard interaction [V]

### 9.1 Display key handling
- `Escape` clears display; `Ctrl+Delete` clears display; Escape with completion popup visible hides the popup.
- Enter/KP Enter solves; `=`/KP Equal solves unless the text is a bare variable name or a function header (then inserts `=`).
- Substitutions (unmodified keys): `*`→`×`, `/`→`÷`, `-`→`−` (with superscript-minus rules), `'`→`′`, `"`→`″`, `>` chains `>>`→`≫`, `>>>`→`⋙`, `<` chains `<<`→`≪`, `×` after `×` becomes `^`.
- Keypad keys work with NumLock off (KP_2 etc. mapped to digits; KP_Decimal inserts locale radix).
- `Ctrl+digit` inserts superscript digit; `Alt+digit` subscript digit (both strip trailing spaces first); number mode returns to normal after any non-digit insertion.
- Ctrl shortcuts: `Ctrl+R` √, `Ctrl+P` π, `Ctrl+T` τ, `Ctrl+I` ⁻¹, `Ctrl+E` ×10 exponent (superscript mode), `Ctrl+F` factorize, `Ctrl+O`/`Ctrl+'` ° (° not in programming), `Ctrl+U` µ, `Ctrl+[`/`Ctrl+]` ⌈/⌉, `Ctrl+minus` ⁻, `Alt+[`/`Alt+]` ⌊/⌋, `Alt+Left/Right` history navigation.
- In programming mode: `Ctrl+B/O/D/H` switch base to binary/octal/decimal/hexadecimal (window-level handler, only active in programming mode; listed in the shortcuts dialog).
- Hex letters A-F/a-f typed in hex base insert digits directly (completion suppressed during).

### 9.2 Global accelerators [V]
Ctrl+C copy (strips separators; whole expression if no selection), Ctrl+V paste (newlines→spaces), Ctrl+Z undo, Ctrl+Shift+Z redo, Ctrl+Escape clear history, Ctrl+comma preferences, Ctrl+W close window, Ctrl+N new window, Ctrl+Q quit (with confirm if >1 window), F1 help, Ctrl+question shortcuts dialog, F10 opens menu. Hardware "Copy"/"Paste"/"Undo"/"Redo" keys mapped too.

### 9.3 Mode accelerators
Ctrl+Alt+B/A/F/P/K(T)/C → basic/advanced/financial/programming/keyboard/conversion.

## 10. Autocompletion [V]

- Popup completion (GTKSourceView) with providers: Built-in Keywords (`in`, `to` — translated), Defined Functions (built-in + custom; label `name()`; details show signature/description), Defined Currencies (code, details "Name - Symbol"; favorites as separate provider with higher priority), Defined Variables (details `name=value`).
- Enabled provider set from `enabled-completions` setting (default: all five). Case-insensitive prefix matching; results sorted (functions by name). Activating completes the typed prefix and, for functions, selects inside `()`. Disabled temporarily during digit/function-name insertion and while a result is displayed.
- `enabled-completions` semantics: values `builtin`, `function`, `currency`, `favorite`, `variable`; preferences expose the currency/favorite subset as a combo.

## 11. Clipboard [V]

- Copy: selection if any, else whole display; thousands separators removed; plain text.
- Paste: text only; newlines become spaces; inserted at cursor replacing selection. In conversion mode, paste replaces the focused converter field.
- Search-provider activation copies results to clipboard (see `08`).

## 12. Accessibility [V]

- Status messages announced via the display's accessible announcement (medium priority); result announced after solving via `=` button (medium priority).
- Variable/function name entries expose invalid state (`INVALID` a11y state) — v51 addition for invalid name entry.
- Bold answer region is a text tag (semantic: previous answer), not separately exposed.
- Standard keyboard navigation; shortcuts window lists all bindings (sections: General, Switching Modes, Programming Mode, Keyboard Entry, Equation).
- High-contrast and dark styles supported via CSS; RTL: keypads/display forced LTR; converter entries LTR with right justification under RTL locale.

## 13. Window state [V]

- Window maximized state and size saved at shutdown; position key exists in schema but is not written by this version (dead key).
- Mode, base, converter category/units, result format, and all preferences saved (see `08` for exact matrix).

## 14. State transitions (summary table) [V]

| Event | Display | History | Settings | Errors |
|---|---|---|---|---|
| Successful solve | result (answer region) | entry appended | — | status cleared |
| Failed solve | unchanged | — | — | status + error style + token selection |
| `=` on result | repeat last op result | entry appended | — | — |
| Digit typed at end of result | display cleared, digit inserted | — | — | — |
| Operator typed after result | answer kept as `_` region | — | — | — |
| C / Escape | display emptied | — | — | status cleared |
| Clear History menu | unchanged | emptied | history-tape cleared | — |
| Mode switch | unchanged (reformatted for base in programming) | shown/hidden per layout | button-mode at shutdown | base label in programming |
| Accuracy/format/grouping change | answer region re-rendered | existing entries re-rendered | bound live | — |
| Base change (programming) | number re-rendered in new base (if plain number) | — | base at shutdown | — |
| Word-size change | displayed integer re-truncated to word size | — | bound live | — |
| Window close | — | history-tape written | size/mode/etc. written | — |
