# 13 — Visual Design Specification

**Purpose:** specify how Radix *looks* — window geometry, layout anatomy, colors, typography, spacing, per-state styling, iconography, and responsive behavior — so that an implementation reproduces the original's appearance. This document extends the scope of `00` (which originally deferred visual design) at the product owner's request; it complements `06` (which specifies *what* each screen contains and how it behaves, toolkit-independently) by specifying *how it is drawn*.

**Fidelity stance.** The visual language is GTK 4 + libadwaita. Wherever possible this spec gives (a) the *style rule* (a formula or token reference that is theme-independent) and (b) a *worked example* — the value measured from the reference screenshot in dark mode. Implementations that use libadwaita get most tokens from the platform by using the named style classes listed here; the app-specific rules below are the part that must be written. Exact rendered colors depend on the libadwaita version, color scheme, and the user's desktop accent choice; the formulas, geometry, and anatomy are the contract.

**Evidence and confidence:** the app's stylesheet and UI templates were read in full [V]; the reference screenshot (`assets/reference-basic-dark.png`, dark style, pink accent variant, basic mode) was pixel-sampled, and measured values below are marked **[M]**. Values derived only from stylesheet semantics without a rendered reference are **[S]**. This document deliberately contains no source-file paths; traceability lives in `01`/`10`/`11`.

---

## 1. Design language and theming

- **Toolkit look [V]:** the application is a libadwaita application. It uses the platform's default widget rendering for buttons, entries, popovers, menus, dialogs, scrollbars, and the header bar, with a small app stylesheet layered on top (specified in this document).
- **Color scheme [V]:** follows the system preference — light, dark, and the two high-contrast variants are all supported. No manual scheme switcher exists in the app.
- **Accent color [V]:** follows the desktop accent setting. The reference screenshot shows the pink accent variant (`=` button, `#d7679d` [M]); any accent must be adopted, never hardcoded.
- **Typography [V]:** the system UI font throughout (no monospace switch). Digits use tabular figures (`tnum` font feature) in the display, history rows, converter values, and the programming bit panel so digits align column-wise.
- **Iconography [V]:** symbolic (monochrome, recolorable) icons from the platform icon theme exclusively; no full-color app icons inside the UI. See §10.
- **App style classes [V]:** the stylesheet only styles elements under the app's own classes: `display-container`, `history-view`, `history-entry`, `sourceview`, `info-view`, `math-buttons`, `number-button`, `operator-button`, `function-button`, `clear-button`, `percent-button`, `parenthesis-button`, `numeric-point-button`, `suggested-action`, `accent`, `bit-panel`, `bit-toggle-button`, `bit-toggle-separator`, `bit-marker-label`, `math-popover`, `action-dialog`, `action-button`, `currency-search-entry`, plus platform classes (`card`, `circular`, `flat`, `dimmed`, `error`, `compact`, `devel` in development builds).

## 2. Color tokens

Reference values measured from the dark-mode screenshot **[M]**; formulas are the normative part **[V]**.

| Token / role | Rule | Dark (measured) |
|---|---|---|
| Window background | theme `window_bg_color` | `#222226` |
| Header bar | theme `headerbar_bg_color` | `#2e2e32` |
| Display card (expression entry area) | theme card background (`.card`) | `#333337` |
| History area | card background **+** `alpha(white, .05)` overlay in dark; `shade(card_bg_color, .95)` in light | `#37373b` |
| Card inner separators | 1 px theme `card_shade_color` | `#27272c` |
| Keypad default button | theme button (no override) | `#38383b` |
| Keypad number button | `alpha(currentColor, .2)` | `#4e4e51` |
| Number button hover / active | `alpha(currentColor, .25)` / `.35` | (not sampled) |
| Accent-filled button (`suggested-action`) | theme `accent_bg_color` | `#d7679d` (pink variant) |
| Foreground / glyphs | theme foreground | `#ffffff` |
| Dimmed label (history `=` symbol) | `dimmed`/dim-label style (≈ 55 % alpha fg) | `#a8a8a9` |
| Insensitive glyph (empty redo) | theme insensitive foreground | `#6c6c6f` |
| Window-close circle | theme windowcontrols button | `#424246` |
| Error text | theme `error` style (red) | (not sampled) |
| Popover background | theme `popover_bg_color` | (not sampled) |

Light and high-contrast modes reuse the identical structure; only the underlying palette changes. The high-contrast variants replace card-shade separators with the stronger `@borders` color [V] (§4.1, §6.4, §8).

## 3. Window and header bar

### 3.1 Window

- **Type [V]:** single `Adw.ApplicationWindow`, title "Calculator". Multiple windows allowed.
- **Default width by mode [V]:** basic **360 px**, conversion **360 px**, advanced / financial / programming / keyboard **700 px** (widths are scale-independent "sp" units, i.e. they scale with the system text scaling factor).
- **Minimum height [V]:** 616 px (`height-request`); window is freely resizable above that; default height lands at ≈ 616 px (screenshot: ≈ 613 px content [M]).
- **Structure [V]:** an `Adw.ToolbarView` with `top-bar-style: raised` (header bar casts a subtle separation shadow onto the content) and a single content area. The content column is wrapped in an `Adw.Clamp` with maximum-size **700 px** and tightening-threshold 700 px — on wider windows, the whole UI column (display + keypad) stays centered at ≤ 700 px [V].
- **Corner radius / shadows [V]:** platform window decorations. The display card casts the standard card drop shadow; in the dark screenshot the shadow is faintly visible under the card over the keypad area [M].
- **Development builds [V]:** get the `devel` style class (platform renders a "Devel" watermark pattern in the header bar). Release builds do not.

### 3.2 Header bar anatomy [V]

| Slot | Control | Look |
|---|---|---|
| Start | Undo button | Flat icon button, `edit-undo-symbolic`. Insensitive (dimmed glyph) when the undo stack is empty. `focus-on-click: false`. |
| Start | Redo button | Flat icon button, `edit-redo-symbolic`, same insensitive rule. |
| Start | Back button | Flat icon button, `go-previous-symbolic`, tooltip "Financial"; **hidden** except while a financial dialog result is being inserted (it pops back to the keypad). |
| Title | Mode selector | A `MenuButton` (not a dropdown list): label = current mode name ("Basic" … "Conversion"), a dropdown arrow always visible next to the label (`always-show-arrow`), can shrink, `focus-on-click: false`, tooltip "Mode Selection". Opens a radio-style menu of the six modes (checkmark on current). |
| End | Primary menu | `MenuButton`, `open-menu-symbolic`, tooltip "Main Menu"; opens the hamburger menu (New Window; Clear History — hidden when action disabled; Result Format section with Automatic/Fixed/Scientific/Engineering radio items; Preferences; Keyboard Shortcuts; Help; About). |

- Window controls (close; minimize/maximize per platform policy) are the platform's windowcontrols — in the dark screenshot a circular button with a close glyph, background `#424246` [M].
- Header-bar buttons use the flat/icon look with no visible borders; hover and active states are the platform's (slight background highlight).
- RTL: start/end slots mirror automatically [V].

## 4. The display stack (card)

### 4.1 Card container [V]

- The display stack is one `.card` container (`display-container`): history view on top, current-expression entry below; the status/info row sits inside the entry area.
- **Margins:** 12 px left/right/top; bottom margin 0 in basic/conversion/advanced/financial/programming (the card sits flush on the keypad area); 12 px bottom only when the card is the last visible element (keyboard mode). In the screenshot the card's bottom edge touches the keypad zone with only its shadow between [M].
- **Corner radius:** 12 px on all corners; **programming mode overrides to 12 px top corners only** (square bottom — the bit panel visually attaches underneath with matching rounded bottom corners, see §6.4).
- **Narrow windows:** inside the 700 px clamp, when width drops below the clamp's small threshold the side margins and radius collapse to 0 (edge-to-edge display) [V].
- **Transitions [V]:** margin changes animate over 200 ms with cubic-bezier(0.25, 0.46, 0.45, 0.94) (mode switching, resize).

### 4.2 History area [V]

- **Background:** card background plus a light overlay (§2) so history reads slightly lighter than the entry area; **1 px bottom border** (`card_shade_color`) separates it from the expression entry; high contrast uses `@borders`.
- **Scrolling:** vertical only; scrollbars are overlay-style; the list is bottom-anchored (`valign: end`) — with few entries they hug the bottom of the area, directly above the entry row (screenshot: the single "2+2 = 4" row sits at the bottom of the empty region [M]). No visible scrollbar until content overflows.
- **Rows** (`history-entry`):
  - **Anatomy:** a 3-part row — expression label (left-aligned, wraps at word-char boundaries, never ellipsized), centered `=` symbol, answer label (right-aligned, ellipsized at the end, max 12 characters wide).
  - **Separator:** 1 px top border per row (`card_shade_color`); the first row's top border is pulled out of view (`margin-top: -1px`) so there is no line directly under the card top.
  - **Padding/typography:** row padding 6 px top/bottom, 3 px sides; font-size **1.3 em**, tabular figures. Expression label additionally has 12 px left / 16 px right outer margins; the `=` symbol gets 12 px horizontal padding and the `dimmed` style (≈ `#a8a8a9` on the dark reference [M]).
  - **Emphasis:** the answer label is **bold**; the expression label is regular weight. Both render in the normal foreground color (screenshot glyphs: pure white [M]).
  - **Interaction:** expression and answer are separately clickable (click inserts that text into the entry — behavior in `06`); rows are not selectable; no hover highlight beyond the platform default for the clickable labels.
- **Empty state:** bare history background, no placeholder text.

### 4.3 Current expression entry [V]

- **Size:** the display widget (`mathdisplay`) has a minimum height of **52 px**; the text view inside requests 45 px, giving the entry row roughly 66 px in practice [M].
- **Text:** font-size **1.4 em**, tabular figures; left/right text margins **14 px**; line spacing 8 px above / 2 px below lines; left-justified; multi-line capable (wraps, scrollable horizontally-never — the scrolled view grows vertically).
- **Background:** transparent — the card background shows through (the entry row is visually just the lower part of the card; screenshot `#333337` [M]).
- **Caret:** platform text caret; visible whenever the display has focus (screenshot after "9+10" [M]).
- **Backspace button:** right of the entry, vertically centered — **circular flat** icon button with `edit-clear-symbolic` (the "⌫" erase glyph), 6 px outer margin, `focus-on-click: false`. No background until hovered.
- **Status/info row:** below the text area — an inline `Adw.Spinner` (12 px left margin; shown while "Calculating…", after the 100 ms delay per `05`) plus a non-editable, word-wrapping text view (12 px side margins, min height 18 px, 4 px bottom padding). This row is invisible when there is no status. Error messages render here with the platform **`error` style** (red text). Successful results do **not** appear here (they go to history) — the row is for spinner/status/error only.
- **Scrollbars:** the horizontal scrollbar, when shown, slides 8 px down while hovered (margin transition) [V].

### 4.4 Autocompletion popup (advanced/keyboard typing) [V]

- Completion window attaches to the entry (platform GTKSourceView-style completion): icon column right-padded 0, details box padded 0 4 px; RTL comment cells get 24 px right margin; the bottom toggle button is collapsed to zero size (invisible). Content itself is specified in `06` §7.2.

## 5. Keypad

### 5.1 Grid geometry [V]

- Buttons are laid out on a homogeneous GTK grid: **5 columns × 64 px wide, rows 44 px high, 4 px spacing** (border-spacing 4 px everywhere). Minimum grid size: 336 × 236 px (5×64 + 4×4, 5×44 + 4×4).
- Outer margins: 12 px on all sides (screenshot: buttons span exactly 38–374 px in a 360 px window, 12 px margins, 4 px gaps, 64 px cells [M]).
- The keypad column is limited by the window's 700 px clamp; multi-grid modes (§6) show two 5-column grids side by side at ≥ 700 px, which is why those default to 700 px (10×64 + 9×4 + 2×12 = 700 [V]).

### 5.2 Button base style [V]

All keypad buttons (including menu buttons): inherit the platform button look — flat dark fill (no gradient), theme corner radius, foreground-colored label; `focus-on-click: false` on every button. App overrides:

| Style class | Rules | Worked example (dark) |
|---|---|---|
| *(default)* | font-size **1.1 em**; padding 2 px 4 px (cancels the platform text-button padding) | fill `#38383b` [M] |
| `number-button` | font-size **1.2 em**, **bolder** weight; background `alpha(currentColor, .2)`; hover `.25`; active/keyboard-activating `.35` | fill `#4e4e51` [M] — visibly lighter than default buttons |
| `operator-button` | font-size **1.6 em** (larger glyph, same default fill) | `#38383b` [M] |
| `suggested-action` | font-size **1.6 em**; platform accent background with white foreground (the `=` button) | `#d7679d` pink accent [M] |
| `clear-button` | font-size **1.2 em**, bolder (the `C` button; default fill) | `#38383b` [M] |
| `function-button` | no color/size override beyond base (mod, %, ( ), ↑n, ↓n, x, f(x), all advanced/financial extras) | `#38383b` [M] |
| `percent-button`, `parenthesis-button`, `numeric-point-button` | no stylesheet rules — hooks only; render as default buttons | `#38383b` [M] |
| `accent` | platform accent style on **toggle** buttons: accent background (white glyph) — the ⇧⁻¹ inverse-modifier toggle and the alternate-function buttons it reveals (sin⁻¹, sinh⁻¹, cos⁻¹, cosh⁻¹, tan⁻¹, tanh⁻¹, y√, yˣ, 10ˣ, eˣ, 2ˣ, ⊼, ⊽, ⊙, ⋙) | accent-tinted [S] |

- **Menu buttons** (`x`, `f(x)` in basic): `MenuButton` with an **upward**-pointing caret (popovers open upward); a CSS rule flips any `arrow` glyph vertically to achieve this.
- **Toggle buttons** (↑n, ↓n, ⇧⁻¹, base/word-size selectors, bit toggles): checked state uses the platform checked rendering (accent-tinted background) so the active modifier is always obvious.
- **Labels [V]:** button captions use Pango markup — subscripts/superscripts via markup (e.g. `y√`, `yˣ`, `10ˣ`, `eˣ`, `2ⁿ√` forms) and italics for variable placeholders; literal glyphs otherwise (`÷ × − + % ( ) C = .`). The full caption inventory per mode is in `06` §6.
- **States:** hover = platform highlight + the number-button formula shift; pressed = stronger fill; insensitive = dimmed glyph; focus = platform focus ring (outline), never a persistent border.

### 5.3 Press feedback [S]

Platform ripple/activation transition only; the app adds none.

## 6. Mode layouts and responsive behavior

| Mode | Keypad structure | Default width | Notes |
|---|---|---|---|
| Basic | one 5×5 grid | 360 px | Reference screenshot §A. |
| Advanced | two 5-column grids side by side (10 columns) | 700 px | 40 function + 10 number buttons + operators row set; accent modifier column. |
| Financial | two 5-column grids side by side | 700 px | 24 function buttons (10 financial formulas + extras); `=` is `suggested-action`. |
| Programming | 18-column grid **plus bit panel** above the keypad | 700 px | 16 number buttons (0–9, A–F); 64-bit toggle panel attaches to the display card (§6.4). |
| Keyboard | **no keypad** — display fills the window | 700 px | Card gets the 12 px bottom margin (it is the last element). |
| Conversion | converter panel (§7) + one 5-column grid | 360 px | Keypad has 25 number-class buttons (digits + unit-value shortcuts), 7 function buttons; C and unit keys; two external-link buttons and an about button on the unit pages. |

### 6.1 Responsive collapse [V]

Advanced, financial, and programming panels are breakpoint containers:

- **≥ 700 px window:** "box" layout — the grids sit side by side as one wide keypad; programming shows the bit panel in its wide form.
- **< 700 px:** "collapse" layout — the same buttons become a **carousel**: one 5-column grid per swipeable page (Adw.Carousel), with the platform's page-position dots below and horizontal swipe/scroll between pages; the panel keeps 12 px margins with no bottom margin (pages continue behind the dots).

This is why the multi-grid modes open at 700 px but degrade gracefully to 360 px windows.

### 6.2 Carousel/paging visuals [V]

Page dots use the platform carousel indicator (small round dots, accent for current). No custom styling.

### 6.3 Keyboard mode [V]

No keypad at all; the display card expands (`vexpand`) to fill the window under the header bar.

### 6.4 Programming bit panel [V]

- **Placement:** directly *below* the display card and *above* the keypad; when the window is wide enough it visually completes the card (display card has rounded top corners, bit panel rounded **bottom** corners, no gap — 12 px side margins inside the clamp).
- **Frame:** 6 px padding; standalone form has a 1 px top border (`card_shade_color`; `@borders` in high contrast); inside the keypad area the border is dropped and padding is 12 px top / 6 px sides.
- **Bit toggles** (`bit-toggle-button`): 64 small flat toggle buttons, one per bit, labeled 0/1; tabular figures, font-size 1.2 em bolder, min-width 20 px, minimal padding (1 px; 0 px in keypad form). On-groups of 1-bits read as accent-checked toggles (platform checked style).
- **Nibble separators** (`bit-toggle-separator`): 10 px right margin (9 px in keypad form) after every 4-bit group.
- **Bit markers** (`bit-marker-label`): small labels marking group boundaries (byte positions), tabular figures, 12 px horizontal margins (0 in keypad form).
- **Narrow windows:** the panel switches to its compact form and the carousel layout takes over the keypad.

## 7. Converter panel (conversion mode) [V]

- **Container:** a second `.card` (`display-container`) above the keypad — same 12 px margins, 12 px radius, card background, 200 ms transition. Padding 12 px left/right/top, 0 bottom.
- **Background:** card shade formula (§2) — slightly tinted relative to the display card so the two cards read as separate surfaces.
- **Category selector:** a `DropDown` (category list), left-aligned, at the panel top; its button is **transparent** unless hovered/active/checked.
- **Value fields:** two editable text views, font-weight **800**, font-size **181 %**, transparent background, 2 px/10 px padding, 4 px bottom margin; the second value sits **36 px** below the first; wrap at char level; context menu (Copy/Paste) on Menu key.
- **Unit selectors:** two searchable `DropDown`s (from-unit, to-unit) below each value field, left-aligned.
- **Status area (currency categories):** replaces the fields when rates are unusable — an `Adw.StatusPage` with the `action-unavailable-symbolic` icon, title "Exchange Rates Not Loaded", body text with an inline **"Refresh rates"** link (compact style), or a full-size `Adw.Spinner` while loading; 12 px side margins, 12 px border spacing.

## 8. Popovers, menus, and dialogs

### 8.1 Math popovers (variables `x`, functions `f(x)`) [V]

- `math-popover` style: **zero** content padding; inner lists pad 6 px and use the popover background; rows are rounded (**9 px radius**, min-height 28 px) with platform hover/active; row buttons padded 2 px 4 px.
- Section headers: bold labels 10 px top / 18 px side margins, separated from the list by a 6 px-margin divider.
- Category navigation: list rows with 10 px horizontal padding and 12 px icon-text spacing; forward/back arrows at **30 % opacity** (80 % in high contrast); a back button row for drilling into categories.
- Search entries and edit rows follow platform entry styling; scrollbars inside popovers get a 26 px-min slider for easier grabbing.
- High-contrast: hovered/active rows get an inset 1 px `@borders` box-shadow.

### 8.2 Header menus [V]

Mode menu and hamburger menu are platform menus (no custom CSS); radio indicators show the active mode/format; "Clear History" hides itself when the action is disabled (`hidden-when: action-disabled`).

### 8.3 Financial dialogs [V]

- Headered modal dialogs (Adw.Dialog): header bar with title, **Cancel** at the start, **Calculate** at the end with the `suggested-action` (accent) style; body is a form of labeled entries.
- The result-insert "back" navigation (§3.2) returns to the keypad; dialogs replace the display context, and their results are not added to history (`04` §4).

### 8.4 Character dialog [V]

- Presented as a **floating sheet** with **18 px** corner radius (`action-dialog`); its action buttons (`action-button`): min-height 24 px, padding 10 px 20 px, **12 px** radius.

### 8.5 Preferences [V]

- Standard Adwaita preferences window: grouped rows, a search entry (margin 3 px 12 px) filtering currency rows, disclosure rows with `go-next-symbolic` trailing icons, switches for toggles.

## 9. Icon inventory [V]

| Icon | Used for |
|---|---|
| `edit-undo-symbolic` / `edit-redo-symbolic` | Header bar undo/redo |
| `go-previous-symbolic` | Financial back button |
| `open-menu-symbolic` | Hamburger menu |
| `edit-clear-symbolic` | Backspace button (entry) and conversion keypad clear |
| `action-unavailable-symbolic` | Converter "rates not loaded" status page |
| `adw-external-link-symbolic` | Conversion keypad: open exchange-rate source links |
| `help-about-symbolic` | Conversion keypad: about/help button |
| `list-add-symbolic` | Function popover: add custom function |
| `document-save-symbolic` | Variable popover: save variable |
| `edit-find-symbolic` | Preferences search |
| `go-next-symbolic` | Preferences disclosure rows |
| caret/arrow glyphs on MenuButtons | Platform-provided; keypad menu buttons flipped to point up |

All are symbolic (recolor with foreground/high-contrast variants). The dropdown carets, checkmarks, radio dots, scrollbar, and window controls come from the platform.

## 10. Motion [V]

Only three animations exist, all toolkit/CSS-level:

1. Display/converter container margin transitions — 200 ms cubic-bezier(0.25, 0.46, 0.45, 0.94) (mode changes, resize collapse).
2. Scrollbar margin slide on hover — 200 ms linear.
3. Platform-provided: popover/dialog open-close, carousel swiping, spinner rotation, button state transitions.

Nothing else animates; results appear in history without motion.

## 11. Accessibility of the visual layer [V]

- All colors come from theme tokens → light/dark/high-contrast guarantees carry over automatically.
- High-contrast gets stronger separators (`@borders`) for history rows, bit panel, and popover rows, and brighter navigation arrows (80 %).
- Tabular figures keep columns readable at any text scale; all fixed sizes specified in px scale via the platform's text-scaling where the widget uses em/sp units.
- Focus is always visible via the platform focus ring (`focus-on-click: false` prevents click-focus stealing it).
- RTL mirrors header slots, popovers, arrows, and text alignment automatically; the keypad grids keep their logical order.

## 12. Reference rendering (normative for dark basic mode)

The provided screenshot is committed as `assets/reference-basic-dark.png` (410×666 including letterbox; window content ≈ 360×613). It shows: dark style, pink accent, basic mode, history containing one entry "2+2 = 4", expression "9+10" with caret, empty redo. All **[M]** values in §2/§4/§5 were measured from it. Where this document and the screenshot disagree on a measured value, the screenshot wins for dark basic mode; where it is silent, the style rules above win.

**Implementation checklist for the GUI (per-mode quick audit):**

1. 360/700 px default widths; 616 px min height; 700 px clamp; raised toolbar.
2. Card margins 12/12/0(/12 keyboard); radius 12 (programming: top-only) + bit panel bottom radius.
3. History: overlay background, bottom border, bottom-anchored rows, 1.3 em tnum, bold answers, dimmed `=`, first-row border hidden.
4. Entry: 1.4 em tnum, 14 px side margins, transparent on card, circular flat backspace right.
5. Keypad: 64×44 cells, 4 px spacing, 12 px margins; number buttons `alpha(currentColor,.2)` 1.2 em bolder; operators/`=` 1.6 em; `=` accent-filled; x/f(x) pop up with up carets.
6. Responsive collapse to carousel < 700 px (advanced/financial/programming).
7. Converter: 800-weight 181 % values, 36 px gap, transparent dropdown buttons, status page with spinner/link.
8. Popovers: 9 px rows, 28 px min height, bold section labels, 30 % arrows.
9. Floating-sheet 18 px character dialog; suggested-action Calculate buttons in financial dialogs.
10. Only the three animations listed in §10.
