# Radix — Handoff Notes

For whoever continues this codebase. Pairs with `REQUIREMENT_COVERAGE.md`
(what is done, with evidence) and `IMPLEMENTATION_DECISIONS.md` (the why,
D1–D25). The specification is `docs/` — and **only** `docs/`: this is a
clean-room reimplementation; never consult the original GNOME Calculator
source, mirrors, or `notes/`.

## Architecture

### radix-core (engine, no GUI deps)

Evaluation pipeline: `solve(canonical, &EvalConfig, &mut EvalState, &UnitCatalog, &CurrencyTable, Option<&mut Budget>) -> SolveOutcome`.

- **lexer/parser** — tokenizes the Unicode surface syntax (digits, base
  prefixes/suffixes, superscripts, roots, vulgar fractions, DMS, `in`
  conversions) and builds the AST straight from doc 02 §5's precedence table
  (D3). Percentage sugar rule in D6.
- **number** — `Number { re, im, exact: Option<Rational>, err }` over `rug`
  (D1/D2). Exact rational shadow for real p/q values; dropped by any
  potentially inexact op. Precision is per-value (`prec()`), 2000 bits in the
  GUI, 1000 in CLI/provider (D5). Attached `err` = "value displayed with
  error" (doc 03 §9) — e.g. `0^0`, Ddb's negative-period status.
- **eval** — EvalConfig { precision, wordlen, angle_unit, base }; scopes for
  user functions; `EvalOutcome.repr_base != 0` iff the result is a base
  conversion (`10 in hex`).
- **solve** — assigns/defines, registers `_` (previous answer), and returns
  `SolveOutcome { value, repr_base, error, stored_variable, defined_function }`.
- **serializer** — the four formats (fixed/automatic/scientific/engineering),
  grouping, base-suffix subscripts, DMS, register strings. All knobs are
  fields: `accuracy, show_thousands, show_zeroes, format, radix, thousands,
  repr_base, parser_base, leading_digits`. Grouping is suppressed when the
  separator equals the radix.
- **units/currency** — category-based conversion with temperature offsets;
  providers IMF/UN/ECB/BOC, first-wins merge (D10 for RSD/XAF fixed rates).
- **financial** — the ten dialog-only formulas. `financial::evaluate(name,
  &args)` is the only entry point; the names are deliberately NOT in the
  function catalog (doc 04 §4: "cannot be performed using the keyboard").
- **persistence/settings** — keyfile at `$XDG_CONFIG_HOME/radix/settings.ini`
  (D13, includes the history tape); `registers` and `custom-functions` under
  `$XDG_DATA_HOME/radix/`; provider caches under `$XDG_CACHE_HOME/radix/`
  (D12). All paths honor XDG env vars (tests rely on that).

### radix-gui (bin `radix`, bin `radix-search-provider`)

- `main.rs` parses `-s/-e/-m/-v`; `--solve` goes through `cli.rs::run_solve`
  **without GTK init**.
- `win.rs` is the window: owns settings/eval state, the solve pipeline
  (`solver.rs` worker thread + 10 ms UI poll, spinner after 100 ms — 10 ms
  so `show_result`'s `set_text` cannot eat characters typed right after
  Return, D25), status
  line, history, programming info line, and the `PadAction` dispatcher. It
  also carries the **visual layer** (doc 13, D22): `load_css` is the entire
  app stylesheet (theme-token formulas: display card, history overlay,
  keypad per-class rules, bit panel, converter, dialogs, and the flat
  header `@define-color headerbar_bg_color @window_bg_color`, D25); the
  shell is `adw::ToolbarView` (raised) → `adw::Clamp` (700) → content, with
  the display card + converter as `.card` children, a MaxWidth-699 px
  breakpoint (< 700 px collapses wide keypads into an `Adw.Carousel`), and
  the header (undo/redo sensitivity, mode radio MenuButton, hamburger
  primary menu, financial back button). The menus are `gio::Menu` models on
  actions (`mode_menu_model`/`primary_menu_model`) with app accelerators —
  D25.
- `keypad.rs` builds the six mode pads; every button posts a `PadAction` via
  the shared sink callback. Wide modes are two 5-column grids
  (`ResponsivePads`)
  that sit side by side ≥ 700 px and reparent into carousel pages below;
  `add_carousel_page` hosts programming's bit-panel page (third page).
  Cell size comes from the `.math-buttons` grid floor — per-button
  minimums must stay dropped (D25).
  Thread-local registries mirror mode toggles (↑n/↓n), the ⇧⁻¹ inverse
  modifier, and the x/f(x) MenuButtons. Programming's base/word-size
  selectors are labelled up-opening MenuButtons in the keypad bottom row
  posting `Base`/`WordSize`; the bit panel is a two-row grid
  (byte markers over 64 toggles grouped by nibble) that re-parents between
  the display card (wide) and its carousel page (narrow) via
  `ProgrammingPads::set_collapsed`.
- `display.rs` is the entry buffer with undo, answer-region tracking, and
  insert modes (super/subscript).
- `converter.rs` is the conversion panel (categories, session-remembered unit
  pairs, currency status stack with "Refresh rates").
- `dialogs.rs`: financial parameter dialogs (replace the display, not added
  to history — doc 04 §4) and the character dialog.
- `bin/radix-search-provider.rs`: gio D-Bus service (D18). `#[path]`-includes
  `../cli.rs` for `normalize_input`; has its own `angle_from_settings`. Keep
  those two in sync conceptually — the module is shared, marked
  `#[allow(dead_code)]` where one binary doesn't use an item.

### radix-cli (bin `radix-cmd`)

`radix_cmd.rs` (REPL + one-shot) + `cli_common.rs` (config, normalization,
`solve_cli` with the UNKNOWN_RATE force-refresh retry, `print_error`).
Error format: MP → `Error <message>`; everything else → `Error <numeric-code>`
where the number is the `ErrorCode` discriminant — that mapping is ABI for
the binary (D19).

## Things that bite

- **repr_base must be applied by the caller.** `SolveOutcome.repr_base` is 0
  (= use current base) or a base-conversion override. The Phase 9 audit found
  all four consumers (GUI finish_solve, `--solve`, radix-cmd, search
  provider) dropping it — `10 in hex` printed `10`. Fixed; keep it that way.
  The canonical rule: `ser.repr_base = if out.repr_base == 0 { config.base }
  else { out.repr_base }`.
- **Settings defaults are load-bearing for tests.** CLI tests run the binary
  with XDG redirected; angle units come from the keyfile, so a stale
  `~/.config/radix/settings.ini` would change `--solve sin(pi)`. Tests are
  hermetic; manual verification should redirect XDG too.
- **`toggle_sign`-style Unicode slicing.** `−` (U+2212) is 3 bytes — never
  slice strings by byte offset for sign stripping; use `strip_prefix`.
- **radix-cmd startup fetches rates** honoring `refresh-interval` (0 =
  cache-only, no network). REPL tests set `refresh-interval=0` in a scratch
  settings.ini to stay hermetic and fast.
- **The search provider's clipboard needs a display.** Without one (headless)
  `gtk::init` fails, clipboard is `None`, and activation still sends the
  notification — the graceful degradation is intentional.
- **GTK `min-width`/`min-height` exclude padding and border.** The keypad CSS
  (`62/42` + `padding: 0` + the 1 px platform border = 64×44 cells, D22) is
  load-bearing: restoring the spec's `min-width: 64px` + `padding: 2px 4px`
  renders 72 px cells and overflows the 336 px basic grid.
- **Bare-X verification is 20 px off.** Without a window manager, GTK's CSD
  shadow margin is inside the X surface: the client area is 20 px smaller
  than the X surface in both axes (measured on gtk4 4.24 / libadwaita
  1.10: a 720×636 surface → 700×616 client, 380×636 → 360×616), so
  `default_width(700)` yields a 690 px client (carousel). The screenshot
  harness resizes to client+20 (720×636 / 380×636 for the 700/360 client
  sizes); on a real desktop the WM sizes the client directly and no
  compensation is needed. Also note a bare Xvfb has no WM: GTK's "exceeds
  window size" warnings mean the window *cannot grow* to its content
  minimum there — a real WM resizes it.
- **`AdwBreakpoint` `MaxWidth` applies at ≤ the value** — use 699 px for doc
  13's "collapse strictly below 700 px" (the breakpoint lives in `win.rs`
  with apply/unapply driving `ResponsivePads::set_collapsed`).
- **The breakpoint transition steals focus — keep `refocus_after_breakpoint`.**
  adw::BreakpointBin saves `last_focus`, hides its child for a frame, and
  restores focus in a tick callback — but a `::focus` query during that
  window cancels the restore and queues a full focus walk, which lands on
  the first focusable widget: the header MenuButtons' internal
  GtkToggleButtons (GTK 4.24 MenuButton wraps an internal toggle). Pressing
  Return then *opens the menu*. The fix (D25) re-grabs the entry 50 ms
  after every apply/unapply — a timeout, not an `idle_add` (the idle raced
  the tick and lost non-deterministically); it skips when a menu is active.
  Removing it re-introduces the bug.
- **Grab focus only after `window.present()`.** `grab_focus` on an unmapped
  window is silently dropped; the pre-present grab left startup focus on
  the first focusable header widget (the other half of the hamburger bug,
  D25).
- **`gtk::Application` does not load libadwaita's stylesheet.** The app
  originally built a `gtk::Application` and never called `adw::init()`: Adw
  widgets worked but rendered unstyled stock-GTK Adwaita (always light,
  boxed buttons) no matter the color scheme. The fix (D23) is building an
  `adw::Application` (`app.rs`); keep it that way.
- **`.history-view list { background: none; color: inherit }` is
  load-bearing.** GTK's ListBox paints its default dark `view` background
  behind history rows; without that rule a near-black band covers the card
  tint (D24). Same family: the dark card is translucent — never replace the
  alpha-based palette with opaque hex values sampled from screenshots.
- **Keypad pages must be direct Stack children — never wrap them in a
  ScrolledWindow.** A scroller reports a near-zero minimum, so the window's
  616 px default never grows to fit and the last keypad rows clip
  (conversion showed exactly this; upstream has no scroller either, D24).
- **`keypad_area`'s vexpand must stay explicitly false.** GTK BoxLayout
  hands children their natural size before splitting the rest among
  expanding children, and expand flags leak through a Stack even from
  *hidden* children (the collapsed-mode carousel's vexpand once stretched
  button rows on every page). The card/converter above the keypad absorbs
  the leftover; the keypad pins to its 236 px minimum.
- **GTK4 Grid counts rows that contain only hidden children toward the
  minimum.** The conversion hex block lived in hidden rows 5–6 and inflated
  the grid minimum to ~343 px (stretched rows). It must stay overlaid with
  the decimal digits in one cell (two nested grids swapped by visibility,
  like upstream `buttons-conversion.blp` — D24).
- **The bit panel lives inside the display card.** `programming()` builds it
  but must NOT `root.append` it — `win.rs` claims it via `card.append`, and
  a widget with two parents silently fails the second append (D24).
- **Never set `GTK_THEME` to verify dark mode.** With `GTK_THEME=Adwaita`
  GTK falls back to its builtin stylesheet and libadwaita never loads — you
  get exactly the unstyled look described above. Dark mode in a bare Xvfb
  session needs the settings portal started *before* the app, after setting
  the scheme: `gsettings set org.gnome.desktop.interface color-scheme
  prefer-dark`, start `/usr/lib/xdg-desktop-portal`, then wait until
  `gdbus call … org.freedesktop.portal.Settings.ReadOne
  org.freedesktop.appearance color-scheme` returns `uint32 1` (a fixed
  sleep raced the backend and the app read "light"). `accent-color pink`
  gives the reference pink (`#d56199`). Debug builds also accept
  `RADIX_DEBUG_ACCENT=<css-color>` to pin the accent without a portal.

## Verification playbook

1. **Engine + CLI**: `cargo test --workspace` (181 tests). Clippy is clean;
   keep it that way (`cargo clippy --workspace --all-targets`).
2. **GUI sessions** (the standing rule: **never** open windows on the user's
   session — always an isolated Xvfb display + private D-Bus):

   ```sh
   Xvfb :98 -screen 0 1280x900x24 &
   dbus-run-session -- bash -c '
     export DISPLAY=:98 GDK_BACKEND=x11
     unset WAYLAND_DISPLAY GTK_THEME        # GTK_THEME disables libadwaita!
     export XDG_CONFIG_HOME=/tmp/radix-verify/config XDG_DATA_HOME=/tmp/radix-verify/data XDG_CACHE_HOME=/tmp/radix-verify/cache
     mkdir -p $XDG_CONFIG_HOME $XDG_DATA_HOME $XDG_CACHE_HOME
     gsettings set org.gnome.desktop.interface color-scheme prefer-dark
     gsettings set org.gnome.desktop.interface accent-color pink
     /usr/lib/xdg-desktop-portal &          # must be up before the app
     sleep 5
     target/debug/radix -m programming &   # or --equation "…", etc.
     sleep 6
     # drive with xdotool (mousemove X Y click 1; type "…"), observe with
     # ffmpeg -f x11grab -video_size 1280x900 -i :98 -frames:v 1 shot.png
   '
   kill %1   # the Xvfb
   ```

   (`import` is broken in this environment; use ffmpeg x11grab. gjs scripts
   need `gjs -m script.gjs`. The staged per-mode harness is
   `/tmp/ai-agent/vis13.sh [all|basic|advanced|prog|conv|fin]`; the D25
   programming/menus harness is `/tmp/ai-agent/vis25b.sh`.)
   Drive the window by name — `xdotool search --name '^Calculator$'`
   (searching `--pid` matches a 1×1 helper surface, not the window); click
   the entry row first or keystrokes never land; sleep ~0.3 s after each
   Return or the next characters race the solve.

3. **Search provider** (isolated session bus, no display needed for query
   tests):

   ```sh
   dbus-run-session -- bash -c '
     target/debug/radix-search-provider &
     sleep 2
     gdbus call --session --dest io.github.hakergeniusz.Radix.SearchProvider \
       --object-path /io/github/hakergeniusz/Radix/SearchProvider \
       --method org.gnome.Shell.SearchProvider2.GetInitialResultSet "['"'"'2+2'"'"']"
     # → ([<'"'"'2+2'"'"'>, <'"'"'open-in-calculator-2+2'"'"'>],)
     # GetResultMetas with the equation id → description " = 4", clipboardText
   '
   ```

   For SP-T005 (clipboard), run under Xvfb and read the clipboard with a
   small GTK script; for the notification, own a mock
   `org.freedesktop.Notifications` first (gjs, REPLACE_EXISTING) so the
   `Notify` call and the 4 s `CloseNotification` withdraw are observable.

4. **Financial dialog** (verified: Ctrm(0.1; 2; 1) → display `7.272540897`,
   dialog closes, result not in history, back button appears): financial
   mode → Ctrm → fill the three entries → Calculate.

5. **Doc 13 visual audit / reference parity**: the staged screenshot harness
   `/tmp/ai-agent/vis13.sh [all|basic|advanced|prog|conv|fin]` runs the whole
   pass on its own Xvfb (`:98`) + private D-Bus under real libadwaita dark
   (portal-first recipe, `RADIX_DEBUG_ACCENT` pinned) and drops PNGs into
   `/tmp/ai-agent/vis13/shots/`: basic reference reproduction (types
   "2+2" ⏎ "9+10" — click the *entry row*, y≈330, not the history area, or
   keystrokes never land; resize to 370×626 first — surface = client + 10),
   advanced wide/narrow, programming, conversion (370×626), financial +
   Ctrm dialog fill (Ctrm at ≈(67, 432) at client 700 with the 44 px keypad
   rows; dialog fields ≈(353, 289/331/373), Calculate ≈(447, 236)).
   Mind the 20 px CSD margin when resizing (surface = client + 20; see
   "Things that bite"). The archived evidence lives in
   `docs/assets/visual-audit/` (D25 added `programming.png` refreshed plus
   `programming-narrow/-mode-menu/-hamburger/-base-menu.png`); pixel checks
   against the reference palette (window #222226, card #3d3d41, entry
   #333337, digits #4e4e51, operators #38383b, accent #d56199) all match.

## Known gaps (see REQUIREMENT_COVERAGE.md for the full matrix)

- **UI-010 autocompletion** — not implemented; `enabled-completions` settings
  key and currency-completion plumbing exist, the popover UI does not.
- **UI-014 accessibility** — no dedicated labels/roles/AT audit pass.
- **Non-C locales (LOC-T001/005)** — D14 ships C locale; the serializer and
  normalization are locale-parameterized, so translation is a data change.
- **Live network currency fetch** — implemented down to the ureq call; never
  exercised against the real endpoint (no network in the build sandbox).
  Offline/staleness semantics are the tested contract (D16).
- **Dogtail-style a11y automation** for doc 09 §8 — those behaviors were
  verified by driving real sessions instead (screenshots + xdotool), not
  bound into automated tests.

## Conventions

- Decisions are append-only in IMPLEMENTATION_DECISIONS.md (extend, never
  renumber; next free id: D27).
- Requirement IDs come from doc 10; keep REQUIREMENT_COVERAGE.md honest —
  statuses there reference the evidence (test names / sessions) directly.
- `cargo fmt --all` + clean clippy before handing off; the crate tree has no
  git history (not a repository) — treat the docs as the history.
