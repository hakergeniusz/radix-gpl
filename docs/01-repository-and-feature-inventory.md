# 01 — Repository and Feature Inventory

**Upstream:** GNOME Calculator master @ `bf92c82b` (2026-10-30), version 51.1, GPLv3+.
**Purpose:** map the repository, inventory subsystems and features, and record which areas need deeper analysis. Source paths in this document exist for Team A traceability; implementation-facing documents (`02`–`09`) stay implementation-independent.
**Confidence markers:** ✔ verified from source read during Phase 1; (p) preliminary — to be confirmed in later phases.

---

## 1. Top-level structure

| Path | Role | Used by the shipped app? |
|---|---|---|
| `lib/` | The calculator engine ("libcalculator"): expression lexing/parsing, numeric types, formatting serializer, units, currencies, financial functions, variables/functions, evaluation budget | **Yes** — static library linked into the app and into engine tests ✔ |
| `src/` | The GTK4 application: window, display, buttons (per mode), converter, history, preferences, popovers, CLI entry `gcalccmd.vala`, main `gnome-calculator.vala` | **Yes** ✔ |
| `src/ui/` | Blueprint UI definitions (compiled to XML templates) + CSS | **Yes** ✔ |
| `data/` | GSchema settings, desktop/metainfo files, man pages, icons | **Yes** ✔ |
| `search-provider/` | GNOME Shell search provider (separate DBus service binary) | **Yes** ✔ |
| `gcalc/` | **GCalc**: independent GObject/Gee-based calculator library (own AST, own parser) | **No** — built by default but not linked by the app ✔ |
| `gci/` | **GCi**: small UI library for GCalc (entry controller) | **No** ✔ |
| `tests/` | Engine unit tests (Vala) + Dogtail GUI tests (Python `.feature`) + gcalc/gci tests | Tests ✔ |
| `libmpfr/` | `mpfr-glue.vala`: GLib-flavored MPFR helpers used by both engines | **Yes** (build detail) ✔ |
| `vapi/` | Vala bindings for GMP/MPFR/MPC | Build detail ✔ |
| `help/` | Mallard user documentation (C + 33 translations) — authoritative user-facing behavior descriptions | Documentation ✔ |
| `po/` | Translations (102 `.po` files) — complete user-visible string inventory | **Yes** ✔ |
| `doc/`, `m4/`, `snap/`, `subprojects/` | API docs scaffolding, autotools remnants, snap packaging, blueprint-compiler meson wrap | Not behavioral ✔ |

Key build facts (verified in `meson.build`, `lib/meson.build`, `src/meson.build`, `tests/meson.build`):

- Language: Vala (compiled via C); deps: GLib ≥ 2.72, GTK4 ≥ 4.17, libadwaita ≥ 1.8.alpha, GTKSourceView-5 ≥ 5.3, libsoup-3 ≥ 3.4, libxml-2.0, gee-0.8 ≥ 0.20, **GMP/MPFR/MPC** (system libraries via custom vapi).
- UI markup: Blueprint (`blueprint-compiler`) compiled to GtkBuilder XML.
- Tests run by `meson test`: `test-equation`, `test-number`, `test-serializer`, `test-currency-providers` (engine), `gcalc-parsing`, `gcalc-solving-basic` (gcalc, not the app engine), `test-gci-entry-controller` (only with `ui-tests` enabled). Dogtail UI tests (`*.feature`) are driven separately.

**Scoping decision (verified):** the observable behavior of "GNOME Calculator" = `lib/` + `src/` + `search-provider/` + `data/`. GCalc/GCI are a parallel engine + demo UI in the same repo and are **out of scope** for Radix. `tests/gcalc-*.vala` results describe GCalc, not the app.

## 2. Subsystem inventory

| Subsystem | Source areas | Size (loc) | Lead documents |
|---|---|---|---|
| Numeric core | `lib/number.vala`, `lib/mpfr-glue.vala`, `libmpfr/mpfr-glue.vala`, `vapi/{gmp,mpfr,mpc}.vapi` | 2,226 | `03` |
| Expression language | `lib/equation.vala`, `lib/equation-lexer.vala`, `lib/equation-parser.vala` | 3,662 | `02` |
| Evaluation orchestration | `lib/math-equation.vala`, `lib/evaluation-budget.vala` | 1,969 | `05` |
| Output formatting | `lib/serializer.vala` | 596 | `07` |
| Functions & variables | `lib/function-manager.vala`, `lib/math-function.vala`, `lib/math-variables.vala` | 938 | `04` |
| Units & conversion | `lib/unit.vala`, `src/math-converter.vala` | 1,518 | `04` |
| Currency | `lib/currency.vala`, `lib/currency-provider.vala`, `lib/providers/{ecb,imf,unt,boc}-provider.vala` | 927 | `04`, `08` |
| Financial functions | `lib/financial.vala`, `src/buttons-financial.vala` | 643 | `04` |
| GUI | `src/*.vala` (12 files), `src/ui/*.blp` (14 files), `src/ui/style.css` | ~7,500 | `06` |
| Settings | `data/org.gnome.calculator.gschema.xml` (17 keys) | 137 | `08` |
| History | `src/math-history.vala`, `src/ui/history-*.blp`, GSchema `history-tape` | ~430 | `06`, `08` |
| Search provider | `search-provider/search-provider.vala` | 382 | `08` |
| CLI | `src/gcalccmd.vala`, `src/gnome-calculator.vala`, man pages `data/*.1` | 472+ | `08` |
| Engine tests | `tests/test-{equation,number,serializer}.vala`, `tests/test-currency-providers.vala` | 2,862 | `09` |
| GUI tests | `tests/{basic,advanced,financial,programming}_mode.feature`, `tests/steps/`, `tests/common_steps.py` | ~1,500 | `09` |
| User help | `help/C/*.page` (31 pages) | 1,595 | all, as behavior evidence |

## 3. Feature matrix

Status legend: **S** = supported/verified present; **S(p)** = present, details pending deeper analysis; **N** = not present; **U** = unverified/unresolved (see `11`).

| Feature | Basic | Advanced | Financial | Programming | Doc |
|---|---|---|---|---|---|
| Arithmetic (+ − × ÷, unary minus) | S | S | S | S | `02`,`03` |
| Percentage | S | S | S | S | `02` |
| Parentheses, precedence | S | S | S | S | `02` |
| Implicit multiplication | S | S | S | S | `02` |
| √, x², x^y, roots | S | S | S | S | `04` |
| Factorial, modulus, integer division, abs | S | S | S | S | `04` |
| Trigonometric + hyperbolic functions | N | S | N | S(p) | `04` |
| Logarithms (ln, log, log₂), exponentials | N | S | N | S(p) | `04` |
| Physical & math constants (π, e, φ, …) | N | S | N | S(p) | `04` |
| User variables (define/reuse/clear) | N | S | S(p) | S(p) | `04` |
| User-defined functions | N | S | S(p) | S(p) | `04` |
| Angle units deg/rad/grad | N | S | S(p) | S(p) | `04` |
| Complex numbers (i) | N | S | S(p) | S(p) | `03` |
| Unit converter (conversion mode + `in` syntax) | N | S | S | N | `04` |
| Currency converter (conversion mode) | N | S | S | N | `04` |
| Base selection 2–16, BIN/OCT/DEC/HEX panel | N | N | N | S | `04` |
| Bitwise AND/OR/XOR/NOT, shifts | N | N | N | S | `04` |
| Word size 8/16/32/64 | N | N | N | S | `03` |
| Rotate/flip bits, endianness swap, complement | N | N | N | S | `04` |
| Character conversion (code ↔ character) | N | N | N | S | `04` |
| Financial term functions (PMT, FV, PV, rate, terms, ctrm, dctrm, sloan?) | N | N | S | N | `04` |
| Live evaluation while typing | S | S | S | S | `05` |
| History view + persistent history | S | S | S | S | `06` |
| Copy/paste | S | S | S | S | `06` |
| Autocompletion popups | S | S | S | S(p) | `06` |
| Keyboard shortcuts / keyboard-only operation | S | S | S | S | `06` |
| Number formats auto/fixed/scientific/engineering | S | S | S | S(p) | `07` |
| Thousands grouping (incl. non-decimal bases) | S | S | S | S | `07` |
| Trailing zeroes display | S | S | S | S | `07` |
| Accuracy setting (digits after point) | S | S | S | S(p) | `07` |
| Localization (102 languages, RTL support) | S | S | S | S | `07` |
| Search provider | S | S | S | S(p) | `08` |
| CLI `--solve` | S | S | S | S(p) | `08` |
| Session/window state restore | S | S | S | S | `08` |
| Date arithmetic | N | N | N | N | `00` |
| Graphing / symbolic solve | N | N | N | N | `00` |

## 4. Feature → source-area map (Team A traceability)

| User-visible feature | Primary source areas |
|---|---|
| Expression entry & evaluation | `lib/equation.vala`, `lib/equation-lexer.vala`, `lib/equation-parser.vala`, `lib/math-equation.vala`, `src/math-display.vala` |
| Result display formatting | `lib/serializer.vala`, `lib/math-equation.vala` (display state) |
| Keypad buttons per mode | `src/ui/buttons-*.blp`, `src/buttons-*.vala`, `src/math-buttons.vala` |
| Unit converter UI + logic | `lib/unit.vala`, `src/math-converter.vala`, `src/ui/math-converter.blp`, `src/ui/buttons-conversion.blp` |
| Currency | `lib/currency.vala`, `lib/currency-provider.vala`, `lib/providers/*` |
| Constants & functions catalog | `lib/function-manager.vala`, `lib/math-function.vala` |
| Variables UI | `src/math-variable-popover.vala`, `lib/math-variables.vala` |
| History | `src/math-history.vala`, `src/ui/history-view.blp`, GSchema `history-tape` |
| Preferences | `src/math-preferences.vala`, `src/ui/math-preferences.blp`, GSchema |
| Shortcuts window | `src/ui/math-shortcuts.blp` |
| Window/menu/mode switch | `src/math-window.vala`, `src/ui/math-window.blp` |
| Base/bit panel | `src/buttons-programming.vala`, `src/ui/buttons-programming.blp` |
| Financial buttons | `src/buttons-financial.vala`, `src/ui/buttons-financial.blp`, `lib/financial.vala` |
| Search provider | `search-provider/search-provider.vala` |
| CLI | `src/gnome-calculator.vala` (option parsing, `--solve`), `src/gcalccmd.vala` (pipe protocol binary) |
| Timeout/guard on evaluation | `lib/evaluation-budget.vala` |

## 5. Features realized indirectly through dependencies

Observable behavior that the original delegates to libraries (contract documented, internals not):

| Behavior | Provided by |
|---|---|
| Arbitrary-precision floats at `precision` bits (default 2000), rounding mode, exponent limits | **MPFR** (GSchema `precision`; vapi `mpfr.vapi`) |
| Complex arithmetic (i, complex results of sqrt/log of negatives) | **MPC** on top of MPFR |
| Exact integer arithmetic; exact rational arithmetic added in v51 ("Keep rational arithmetic exact (using gmp)") | **GMP** (mpz/mpq) |
| C math library single-precision/double-precision functions where used | libm (`cc.find_library('m')`) |
| Locale decimal/thousands separators, digit grouping | GLib/POSIX locale data |
| HTTP fetch of currency rates, XML parsing | libsoup-3, libxml-2.0 |
| Syntax highlighting & inline completion UI in entry field | GTKSourceView-5 |
| UI templates, CSS, actions | GTK4 + libadwaita |
| Settings storage | GSettings/dconf |

## 6. Tests inventory (evidence base for `09`)

| Test | Covers | Nature |
|---|---|---|
| `tests/test-number.vala` (1,160 loc) | `Number` arithmetic: ops, roots, powers, trig, factorial, bitwise, conversions, serialization edge cases | Headless unit assertions |
| `tests/test-equation.vala` (1,103 loc) | End-to-end expression evaluation and error cases | Headless unit assertions |
| `tests/test-serializer.vala` | Number → string formatting | Headless unit assertions |
| `tests/test-currency-providers.vala` | Parsing of provider rate files from `tests/` fixtures | Headless, fixture files |
| `tests/*_mode.feature` (4 files, Dogtail) | Keypad interaction per mode: clicking buttons produces expected display text | GUI (requires display) |
| `tests/gcalc-*.vala` | **GCalc only** — not evidence about the app engine | Out of scope |

Fixture files: `tests/rms_five.xls` (currency provider sample data).

## 7. Features with special status

- **Deprecated/removed recently (verified via NEWS):** `ones` (one's complement) named function removed in 50.0 (replaced by dedicated button); date arithmetic mode removed upstream in earlier releases (not in this tree).
- **New in 50–51 (behavioral risk if Radix copies older releases):** persistent history tape (GSchema `history-tape`, !469); exact rational arithmetic via GMP (!470); digit grouping in non-decimal bases (!433); swap-endianness button (!452); exponent displayed in current base (#536); favorite currencies + enabled-completions settings; a11y announcements for invalid variable names (!471).
- **Experimental/unverified:** completion trigger semantics; engineering format behavior in non-decimal bases; behavior of `refresh-interval` = 0 (documented as "won't fetch"); mobile/narrow-layout behavior (libadwaita adaptive states in `.blp` files). Tracked in `11`.
- **Platform-dependent:** 32-bit builds disable unsigned right shift of 32-bit words (NEWS 49.1.1, #503) — Linux x86_64 target assumed for Radix unless stated.

## 8. Glossary

| Term | Meaning in this project |
|---|---|
| **Equation** | The complete user-entered expression string plus its evaluation state; the original names its central object "equation" even when it is a bare expression. |
| **Display** | The entry + result area (a GTKSourceView) where the user types and the live result is shown. |
| **History tape** | Persisted list of (equation, answer) pairs shown in the history view. |
| **Mode** | Keypad layout: basic, advanced, financial, programming. |
| **Word size** | Bit width (8/16/32/64) used for integer/bitwise semantics in programming mode. |
| **Accuracy** | Number of digits displayed after the decimal point (GSchema `accuracy`, default 9). |
| **Precision** | Internal MPFR bit precision (GSchema `precision`, default 2000). |
| **Serializer** | Engine component converting an internal number to display text (formatting rules in `07`). |
| **Angle unit** | deg/rad/grad interpretation for trig functions. |
| **Source/target units** | The two sides of the converter panel. |
| **Provider** | An online source of currency exchange rates. |
| **GCalc / GCi** | Separate engine/UI libraries in the same repo, not used by the app (out of scope). |
