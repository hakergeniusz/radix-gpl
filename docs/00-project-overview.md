# 00 — Project Overview

**Project:** Radix — behavioral specification of GNOME Calculator
**Upstream analyzed:** GNOME Calculator, repository `https://gitlab.gnome.org/GNOME/gnome-calculator`, branch `master`, HEAD commit `bf92c82b98493489850932062ef1f594174e4134` (2026-10-30), version string **51.1**
**Audience:** Team B, the independent Rust implementation team. This document set is the functional reference; Team B does not need the original source.
**Status of this document set:** see `INDEX.md`. Evidence/confidence vocabulary is defined in `10-requirements-and-traceability.md` §2.

---

## 1. Product purpose

GNOME Calculator is a desktop calculator application that solves mathematical equations typed or keyed in by the user, converts values between units and currencies, and converts numbers between positional bases. It is designed to be a default calculator application in a desktop environment. It is not a symbolic algebra system, a spreadsheet, or a programming language; it is an interactive expression evaluator with a graphical keypad, a persistent history, converters, and a small set of user-definable functions and variables.

The product name in the user interface is "Calculator" (window title "Calculator"; the development build is "(Nightly) Calculator"). The application id is `org.gnome.Calculator` (`org.gnome.Calculator.Devel` for development builds).

## 2. Main user workflows

The application supports four user-facing workflows, all reachable in a single window:

1. **Expression calculation (primary workflow).** The user types or clicks an arithmetic/scientific/programming expression into the display, which evaluates it live while typing; pressing Enter (or `=`) commits the result into the calculation history. The result can be reused in further calculations.
2. **Unit conversion.** A dedicated conversion view (conversion mode) converts a value between two units of the same physical category (angle, area, currency, data, duration, energy, length, mass, pressure, speed, temperature, time, volume). Conversions can also be typed directly into any mode using `in`-syntax expressions. Currency rates are fetched from network providers.
3. **Base conversion (programming mode).** A fixed panel shows the current operand simultaneously in binary, octal, decimal and hexadecimal, and digit buttons are enabled/disabled according to the selected base. Unit conversion is unavailable in programming mode; the base conversions replace it.
4. **History review.** A history view lists past calculations as equation/answer pairs; entries can be copied, and history can be cleared. As of version 51 the history persists across sessions.

Secondary workflows: defining variables and functions via popovers/preferences, setting preferences (precision, format, angle units, etc.), copying results, invoking the calculator from the GNOME Shell search provider or from the command line (`gnome-calculator --solve` and the `gcalccmd`-style `--equation`-style invocation; verified in `08`).

## 3. Calculator modes

The application has four keypad modes (GSchema enum `org.gnome.calculator.ButtonMode` also contains a `keyboard` value, which is not a user-selectable keypad mode; see `06`):

| Mode | Purpose | Distinct capabilities |
|---|---|---|
| **Basic** | Simple arithmetic | Digits, basic operators, percentage, parentheses, √, clear, backspace. No angle/measure conversion panel. |
| **Advanced** | Scientific | Everything in Basic plus trigonometric/hyperbolic functions, logarithms, powers, roots, factorial, modulus, absolute value, user variables, physical constants, unit conversion panel, angle-unit selector. |
| **Financial** | Financial math | Basic arithmetic plus financial term buttons (interest, period payment, present/future value, etc. — full catalog in `04`) and the conversion panel. |
| **Programming** | Base/bit operations | Base selector (BIN/OCT/DEC/HEX), 64-bit word-size representation panel showing the current value in all four bases, bit-rotation and bit-flip buttons, one/two's complement operations, swap-endianness button; character conversion. Unit conversion panel is not available. |

Mode is changed from a menu in the header bar; the last used mode is persisted.

## 4. User-visible feature list

Verified to exist in the analyzed source (details in the referenced documents):

- Live evaluation of expressions while typing (`02`, `05`).
- Arithmetic, scientific, bitwise, and financial operators and functions (`04`).
- Named constants (π, e, φ etc.) and physical constants (full catalog in `04`).
- User-defined variables and user-defined single-assignment functions (`04`).
- Angle units: degrees, radians, gradians (`04`).
- Unit conversion across ≥ 12 categories with hundreds of units (`04`).
- Currency conversion with rates from online providers (ECB, IMF, UN, Bank of Canada) (`04`, `08`).
- Number bases 2–16 in programming mode; hex/binary/octal literals in expressions (`02`).
- Word sizes 8/16/32/64 bits with signed/unsigned semantics for bitwise operations (`03`, `04`).
- Number formatting: automatic/fixed/scientific/engineering formats, digit grouping, trailing zeroes, adjustable display accuracy (`07`).
- Calculation history with persistence across sessions (`06`, `08`).
- Copy/paste of expressions and results (`06`, `07`).
- Completion popups for variables, functions, currencies while typing (`06`).
- Preferences dialog (accuracy, angle units, number format, thousands separators, trailing zeroes, word size, currency refresh interval, currency display) (`06`, `08`).
- Keyboard shortcuts and full keyboard-driven operation (`06`).
- GNOME Shell search provider (`08`).
- Command-line solving (`08`).
- Accessibility: Orca/screen-reader labels, high-contrast support, keyboard navigation (`07`).

## 5. Runtime and platform assumptions

- Original implementation platform: GNU/Linux, GNOME desktop, GTK 4 (≥ 4.17), libadwaita (≥ 1.8.alpha), GTKSourceView 5 (used for the expression entry widget with syntax highlighting and completion).
- Numeric libraries: **GMP** (integer/rational), **MPFR** (floating point), **MPC** (complex). These define the numeric semantics of the original to a degree that matters for fidelity (`03`).
- Network (libsoup 3): only for currency rates. The application is fully functional offline except live currency conversion.
- Localization: gettext; ~102 translation files ship; all user-visible strings are translatable (`07`).
- Desktop integration assumed by the original: GSettings/dconf, desktop notification-free, single-instance activation, MIME/desktop entry, GNOME Shell search provider, Yelp help documents.

The Radix replacement is expected to reproduce *behavior*, not the toolkit; `06` specifies interactions toolkit-independently.

## 6. External integrations (observable)

| Integration | Observable behavior | Document |
|---|---|---|
| GSettings schema `org.gnome.calculator` | All preferences and history persistence; writable by external tools | `08` |
| Currency rate providers (HTTP) | ECB, IMF, UN ("unt"), Bank of Canada; refresh interval setting; offline fallback to bundled rates | `04`, `08` |
| GNOME Shell search provider | Querying "Calculator" in Shell search evaluates simple expressions and returns up to N results | `08` |
| Command line | `gnome-calculator --solve "expression"` prints result and exits; `--version`, `--mode`, debug options | `08` |
| Desktop entry / metainfo | Single window app, opens in last mode/size/position | `06`, `08` |
| Manual pages | `gnome-calculator(1)`, `gcalccmd(1)` | `08` |

## 7. Scope of the Radix replacement

In scope (must reproduce):

- The expression language, evaluation semantics, and numeric behavior of the calculator engine (`02`, `03`, `04`, `05`).
- All four calculator modes' user-visible behavior (`06`).
- Number formatting and localization behavior (`07`).
- Persistence behavior defined by the GSchema keys (`08`).
- CLI solving and search-provider query behavior as specified (`08`).

Out of scope for behavioral fidelity (documented only for completeness):

- The separate **GCalc** library and its tests. GCalc is a GObject-based calculator library shipped from the same repository but **not used by the GNOME Calculator application** (verified from the original build configuration — the application links only the engine library, and the GCalc tests build separately; see traceability doc 01). Radix does not need to reproduce it.
- Pixel-exact visual fidelity is not guaranteed; the visual layer is specified to the token/geometry level in `13` (colors as style formulas + measured dark reference, 64×44 keypad geometry, anatomy, icons, responsive collapse), with only platform-provided chrome left unspecified.
- The original's internal architecture (Vala classes, GObject signals, widget tree).
- Wayland/X11 specifics beyond what `06`/`08` specify as observable behavior.
- A thin in-tree GLib wrapper around the arbitrary-precision math library — an implementation detail of the original (recorded in traceability doc 01).

## 8. Known compatibility constraints

- **Numeric core.** The original computes with MPFR at a user-configurable internal precision (default **2000 bits**) and with MPC for complex numbers, plus GMP rationals for exact results where applicable (v51 "keep rational arithmetic exact" change). Reproducing observable results (e.g. why `1/3*3 == 1`, how many digits a result shows) requires equivalent precision semantics (`03`).
- **Locale sensitivity.** Expression input accepts localized decimal separators and thousands separators; output formatting is locale-dependent (`02`, `07`). A replacement must support at least the locale categories the original does.
- **Currency providers** change over time (URLs, formats); the spec pins observable contract (file format parsed, refresh behavior), not provider internals (`04`, `08`).
- **GTK-sourceview entry**: autocompletion behavior is part of UX spec (`06`) but its exact trigger timing is only partially verifiable without running the app (`11`).

## 9. Features explicitly out of scope (not present / not required)

The following are **not** features of GNOME Calculator 51.x and are not required of Radix (verified by absence from source and help):

- Graphing/plotting.
- Symbolic algebra / equation solving for unknowns (e.g. "solve for x").
- Date arithmetic (calculator modes for dates were removed upstream before v46; not present in this tree).
- Matrix or vector arithmetic.
- Scripting language, loops, conditionals.
- Custom themes or skins.
- Saving/loading calculation sessions to files.

Items whose status could not be fully verified from source alone are listed in `11-open-questions-and-uncertainties.md`.
