# 08 — State, Persistence, and Integration

**Scope:** every persisted setting and file, session state, startup/shutdown behavior, command-line interface, search provider, and desktop integration. UI behavior of these surfaces is in `06`.
**Evidence:** direct source analysis of the settings schema, application lifecycle and shell code, CLI entry points, function/variable/currency management code, man pages, and the Flatpak manifest. Confidence: **V** / **S** / **I**.

---

## 1. GSettings schema `org.gnome.calculator` [V]

Path `/org/gnome/calculator/`. All keys, types, defaults, and write moments:

| Key | Type | Default | Purpose | Written when |
|---|---|---|---|---|
| `history-tape` | a(ss) | [] | Persisted history (equation, answer) pairs | Window close (all entries); cleared by Clear History |
| `accuracy` | i | 9 | Digits after radix displayed | Live-bound (preferences change immediately) |
| `word-size` | i (8..64) | 64 | Bit width for bitwise ops | Live-bound |
| `base` | i (2..16) | 10 | Numeric base | Shutdown (programming keypad base) |
| `show-thousands` | b | false | Digit grouping | Live-bound |
| `show-zeroes` | b | false | Trailing zeroes | Live-bound |
| `number-format` | enum | automatic | automatic/fixed/scientific/engineering | Live-bound (menu radio) |
| `angle-units` | enum | degrees | radians/degrees/gradians | Live-bound |
| `refresh-interval` | i | 604800 | Currency refresh seconds; 0 = never fetch | Preferences change (immediate re-refresh) |
| `currency-display` | enum | name | code/name/both | Preferences change |
| `enabled-completions` | as | [builtin, function, currency, favorite, variable] | Completion providers active | Preferences change |
| `favorite-currencies` | as | [] | Favorite currencies | Preferences change |
| `button-mode` | enum | basic | basic/advanced/financial/programming/keyboard/conversion | Shutdown |
| `unit-category` | s | 'angle' | Converter category | Shutdown |
| `source-units` | as | ['degree'] | Converter from-units (per category map) | Shutdown |
| `target-units` | as | ['radian'] | Converter to-units | Shutdown |
| `precision` | i | 2000 | Internal MPFR bits | Read at startup (dconf-only; no UI) |
| `window-position` | (ii) | (-1,-1) | Last window position | **Not written by this version** (dead key) |
| `window-maximized` | b | false | Last window maximized | Shutdown |
| `window-size` | (ii) | (-1,-1) | Last window size | Shutdown |

Notes [V]: `word-size` schema range min=8 max=64; `base` min=2 max=16 (v49 "Only allow 2 to 16 number bases"). `accuracy` has no schema range. Delayed settings apply() at shutdown for the shutdown-written keys; live-bound keys write immediately.

## 2. Files [V]

| File | Format | Purpose |
|---|---|---|
| `$XDG_DATA_HOME/gnome-calculator/registers` | `name=value` lines; value in scientific format, radix `.`, up to ~50 fractional digits, exponent as `×10¹²`-style superscript | User variables; re-saved on every set/delete |
| `$XDG_DATA_HOME/gnome-calculator/custom-functions` | `name(p1;p2)=expr@description` lines | User functions; re-saved on every add/delete |
| `$XDG_CACHE_HOME/gnome-calculator/eurofxref-daily.xml` | ECB XML | Currency rates cache |
| `$XDG_CACHE_HOME/gnome-calculator/rms_five.xls` | IMF TSV-in-XLS | Currency rates cache |
| `$XDG_CACHE_HOME/gnome-calculator/un-daily.xls` | UN CRLF TSV | Currency rates cache |
| `$XDG_CACHE_HOME/gnome-calculator/fxtwdcad.xml` | Bank of Canada XML | TWD rate cache |

History is NOT a file: it lives in GSettings (`history-tape`). Session-only state: current expression, answer value, undo/redo stacks, mode runtime state — lost on close (only history + settings persist).

## 3. Application lifecycle [V]

- Multiple windows per process and multiple processes allowed (`NON_UNIQUE`); each window has its own evaluation state (own answer/undo state, shared variables/functions/formatting settings).
- Shutdown of the app writes settings from the **last opened window** only (mode, base, converter state, window size/maximized). History tape is written per window close request.
- Currency refresh runs at startup (async) and when refresh-interval changes; a synchronous forced refresh occurs when a conversion hits `UNKNOWN_RATE` in CLI mode.

## 4. Command line — `gnome-calculator` [V]

Options (getopt; man page `gnome-calculator(1)`):

| Option | Effect |
|---|---|
| `-s, --solve EQUATION` | Evaluate and exit. Does not open a window. |
| `-e, --equation EQUATION` | Open window with the equation loaded (multi-line; blank lines solve). Escape sequences in the argument are unescaped first. |
| `-m, --mode MODE` | Start in mode: basic, advanced, financial, programming, keyboard, conversion. |
| `-v, --version` | Print `<program-name> <version>` to **stderr**, exit 0. |
| `-h/--help` | Standard help. |

`--solve` details [V]:
- Input normalization: locale thousands separator removed (unless it is `.`); locale radix replaced by `.`.
- Evaluation context: base 10, **word length 32**, angle units from settings. Precision default (no GSchema precision read in this path — library default 1000? — **verified**: precision is a library static default 1000 in this path; the app-window path sets 2000. `--solve` results may therefore differ in last digits from GUI results for long computations).
- On `UNKNOWN_RATE`: currency rates force-refreshed (sync) and the parse retried once.
- Output: result printed to stdout with trailing newline, AUTOMATIC format, accuracy 9, representation base applied. Output-formatting errors go to stderr (exit 1).
- Errors: `MP` → `Error: <message>` (stderr, exit 1); other error codes → `Error: ErrorCode.<NAME>` (e.g. `Error: ErrorCode.INVALID`), exit 1.
- No GUI interaction; works headless.

## 5. Command line — `gcalccmd` [V]

A separate console calculator binary (man page `gcalccmd(1)`).
- `gcalccmd "expr"`: solves the single argument (same normalization as --solve; base 10, wordlen 32, **always degrees**).
- No arguments: REPL. Prints `> ` prompt; each line solved; `exit`, `quit`, empty line, or EOF exits. Errors: `Error <message>` (MP) or `Error <numeric-code>` (integer of the enum) to stderr. Results to stdout, AUTOMATIC format, accuracy 9.
- Currency loaded synchronously at startup (no async); refresh-interval respected.
- The REPL does NOT print the prompt line for EOF exits and appends a newline if input ended without one.

## 6. GNOME Shell search provider [V]

DBus service `org.gnome.Calculator.SearchProvider` implementing `org.gnome.Shell.SearchProvider2` (activation via D-Bus service file; desktop app ID org.gnome.Calculator).

- **Query handling:** search terms joined by spaces = candidate expression. Rejected without results when: the normalized text parses as a plain number, or contains `=` (variables/functions never mutated from search), or fails to evaluate, or hits the evaluation budget.
- Normalization: thousands separator removed, locale radix → `.`. Evaluation: base 10, wordlen 32, angle units from settings, budgeted (50 ms deadline; limits in `05` §5). Result rendered AUTOMATIC accuracy 9 with representation base.
- At most **one** result: identifiers `[equation, "open-in-calculator-" + equation]`. Result metadata: name = equation, description = ` = <result>`, icon `edit-copy-symbolic`, clipboardText = result; second entry "Open in Calculator" with app icon.
- Activation: result click → result copied to clipboard, desktop notification "Result copied" / "Result was copied successfully" (withdrawn after 4 s). "Open in Calculator" → spawns `gnome-calculator --equation <terms>`.
- Results cached (last 10 equations) for the service lifetime; the service idles out after 10 s inactivity.
- A `--solve`/`--angle-units` CLI is also implemented in the provider binary (internal/testing).

## 7. Desktop integration [V]

- Desktop entry: `org.gnome.Calculator` (Development variant `org.gnome.Calculator.Devel`); StartupWMClass matches; categories include Utility/Calculator; DBusActivatable.
- Metainfo: app description, screenshots, releases; OARS; provides `org.gnome.Calculator.desktop`.
- Help: `help:gnome-calculator` URI (Yelp Mallard documents, 31 pages in C + translations).
- Man pages: `gnome-calculator(1)`, `gcalccmd(1)`.
- Flatpak (org.gnome.Calculator.Devel.json): GNOME runtime; bundles mpfr/gmp/mpc; uses libsoup3; development app id and nightly metadata.
- Snap packaging present (snap/snapcraft.yaml).
- Icons, CSS theming per libadwaita; GSettings schema installed/compiled at install time.
- Environment: no special environment variables consumed (locale via standard `setlocale(ALL, "")`); debug logging via GLib `debug()` for currency updates (G_MESSAGES_DEBUG).
- Failure modes: no dconf → settings fall back to defaults (standard GSettings behavior); no network → currency rates unavailable (conversion errors; converter shows "Exchange Rates Not Loaded"); no display → `--solve` and `gcalccmd` work headless, GUI fails with standard GTK error.
