# 10 — Requirements and Traceability

**Purpose:** stable, testable requirement IDs covering every documented behavior, each with evidence and confidence, linked to tests in `09`.
**Evidence vocabulary (used across all documents):**
- **Verified** — direct code-path evidence, corroborated by upstream tests and/or multiple independent sources (help/tests/source agree).
- **Strongly supported** — clear implementation path, not independently corroborated (or only partially testable headlessly).
- **Inferred** — plausible interpretation of incomplete evidence; must not be relied on without confirmation.
- **Unknown** — insufficient/contradictory evidence (tracked in `11`).
- Runtime execution of the original application was **not available** during analysis; "Verified" therefore always means source/test-verified, never experimentally verified on a running app. Where a distinction matters it is called out.

Confidence column: V = Verified, S = Strongly supported, I = Inferred.

---

## EXPR — Expression language

| ID | Requirement | Evidence | Conf | Tests |
|---|---|---|---|---|
| EXPR-001 | Tokens: operators/symbols per `02` §2, including Unicode variants (−, ×, ÷, ≪/≫/⋙, bitwise symbols, sup/sub digits, vulgar fractions, DMS marks); whitespace ignored; unknown characters are errors. | lexer source; tests | V | EXPR-T031..038, LIT-T009 |
| EXPR-002 | Digits: any Unicode Nd digit accepted with Unicode value; A–F hex letters; fullwidth hex accepted. | lexer + GLib classification verified; Arabic-digit tests | V | LIT-T013/014 |
| EXPR-003 | Literals: base suffixes `₂`–`₁₆` override parser base; prefixes `0b/0o/0x` case-insensitive; prefix+suffix must agree; digits above base are errors. | parser+number source; tests | V | LIT-T001..008 |
| EXPR-004 | Vulgar fractions and DMS `d°m′s″` literals per `02` §3. | source; tests | V | LIT-T009..012,015 |
| EXPR-005 | Identifiers: word = letters/underscore/backslash + optional subscript digits; classification order operator→function→currency→unit→variable; undefined words become products of single-char variables; `in`/`to` matched against translation. | lexer/parser source; tests | V | VAR-T001..004, UNIT-T016..018 |
| EXPR-006 | Precedence/associativity table per `02` §5, including implicit multiply at multiply level, right-assoc power, postfix %, factorial highest. | parser source; precedence tests | V | EXPR-T001..014 |
| EXPR-007 | Percentage: postfix = /100; additive sugar `a±b%` = a·(1±b/100) for simple right sides; compound right sides behave per asserted quirks. | parser source; tests | V | EXPR-T015..022 |
| EXPR-008 | Superscript/subscript integer powers and nth roots; zeroth root errors; function superscripts (`sin²`, `sin⁻¹` dispatch). | parser source; tests | V | EXPR-T023..028, FUNC-T005 |
| EXPR-009 | Grouping: `()`; `[]` round (ties away); `{}` fractional part (x−floor x); `⌊⌋`/`⌈⌉`/`‖`; nesting resets depth. | parser source; tests | V | NUM-T013/014, EXPR-T004 |
| EXPR-010 | Bitwise ops require non-negative integers; AND/OR/XOR unmasked; NOT/NAND/NOR/XNOR masked to word size; nibble-wise semantics. | number source; tests | V | NUM-T024..026 |
| EXPR-011 | Shifts: count mod word size; sign rules per `02` §7.6; overflow checks; errors for non-integers. | number source; tests | V | NUM-T027/028 |
| EXPR-012 | Conversions `in/to`: unit conversions require a from-unit (`x unit1 in unit2`); representation-base conversions only at end-of-expression; currency conversions with mandatory from-currency; errors UNKNOWN_UNIT/CONVERSION/RATE. | parser source; tests | V | UNIT-T001..023 |
| EXPR-013 | Assignment `name=` stores variable (result displayed); constants protected silently; single `=` only. Function definition `f(p;…)=expr[@desc]` with alphabetic names; redefinition rules; INVALID for built-in collisions and comma args. | parser/function-manager source; tests | V | VAR-T005..013 |
| EXPR-014 | Canonicalization: display→parser replaces answer with `_ `, strips separators, radix→`.`; `\\cdot` literal multiply. | math-equation source; tests | V | UI-T014/015 |
| EXPR-015 | Error classes per `02` §10 with user messages per `05` §4. | math-equation source | V | CLI-T002/003, UI-T002 |
| EXPR-016 | Partial input never evaluated for display; commit triggers evaluation. | math-equation/display source | V | UI-T001..004 |

## NUM — Numeric semantics

| ID | Requirement | Evidence | Conf | Tests |
|---|---|---|---|---|
| NUM-001 | Values are MPFR/MPC complex at configurable precision P (default 2000 bits, GSchema `precision`), round-to-nearest. | number.vala; gschema; app wiring | V | NUM-T022 |
| NUM-002 | Exact rational shadow: +,−,×,÷ on rationals are exact (0.1+0.2=0.3; 1÷3×3=1; 19-digit fractions cancel); dropped by irrational ops. | number.vala rational code; v51 NEWS; tests | V | NUM-T001..005 |
| NUM-003 | Division by zero (incl. 0.0/0.0) is an error, never inf. | number.vala; dogtail | V | NUM-T006, UI-T002 |
| NUM-004 | 0^0 and 0^negative are errors returning 0 with error state. | number.vala; tests | V | EXPR-T029/030 |
| NUM-005 | Powers: reciprocal-integer exponents → root path; complex power for negatives; root sign rules (odd real / even complex). | number.vala; tests | V | FUNC-T015..020 |
| NUM-006 | ln/log domains and complex branch (+πi for negatives); log base rules; log_N subscript functions. | number.vala; tests | V | FUNC-T012..014, T022 |
| NUM-007 | Trig exactness guards at axis angles per angle unit; undefined tangent at 90°+180°n; inverse domains; angle conversion factors π/180, π/200. | number.vala; tests | V | FUNC-T001..009 |
| NUM-008 | Factorial: 0!=1; negative integer and complex errors; non-integers via Γ(x+1). | number.vala; tests | V | NUM-T008..011 |
| NUM-009 | mod semantics per `03` §4.1 (sign adjustments); modexp repeated-squaring. | number.vala; tests | V | NUM-T012 |
| NUM-010 | Rounding family: round ties-away-from-zero; floor/ceil; `{}` floor-based vs `frac` trunc-based. | parser/number source; tests; MPFR verified | V | NUM-T013/014/015 |
| NUM-011 | Word-size reinterpretation on word-size change (two's complement); floats exempt; bit panel IEEE-754 bit view for floats 32/64. | math-equation source | V | UI-T011, UI-T010 |
| NUM-012 | `rand` fresh uniform [0,1) per read; `_` previous answer. | math-equation source; help | V | NUM-T030, VAR-T015 |
| NUM-013 | Negative zero normalized; arg/ln of negative reals return +πi branch. | number.vala; NEWS; tests | V | NUM-T018, FUNC-T021/22 |
| NUM-014 | Infinity/NaN display as ∞/−∞/NaN (float bit input only). | serializer | V | FMT-T017 |

## FUNC — Functions, constants, units, financial

| ID | Requirement | Evidence | Conf | Tests |
|---|---|---|---|---|
| FUNC-001 | Built-in function catalog per `04` §1 (names, aliases incl. asin-style and `name⁻¹`, argument counts and error messages). | function-manager/math-function source; tests | V | FUNC-T*, VAR-T012 |
| FUNC-002 | Constants catalog per `04` §2 with exact values; protected from assignment; Cyrillic `е` quirk. | parser source; help variables.page | V | FUNC-T017, VAR-T006 |
| FUNC-003 | User functions: definition syntax, positional binding, shadowing, persistence file, name rules. | math-function source; tests | V | VAR-T007..014 |
| FUNC-004 | User variables: `registers` file persistence, set/delete, popover rules. | math-variables source | V | STATE-T004, UI-T017 |
| FUNC-005 | Unit catalog per `04` §3.1 with exact conversion expressions, categories, symbols, ambiguity rule (multi-match → no unit). | unit.vala; tests | V | UNIT-T001..018 |
| FUNC-006 | Temperature offset conversions per `04` §3.1. | unit.vala; tests | V | UNIT-T004 |
| FUNC-007 | Currency: 67 built-in currencies; provider order and rate formulas per `04` §3.2; UNKNOWN_RATE offline; refresh-interval semantics incl. 0=never. | currency sources; gschema | V | UNIT-T022/023, STATE-T007 |
| FUNC-008 | Financial functions: the 10 dialogs with formulas/fields per `04` §4; results replace display, not history; dialog entry validation. | financial.vala; buttons-financial source | V | UI-T019 |
| FUNC-009 | Factorize: prime factorization ascending, sign on first factor, integer-only error. | number/math-equation source; tests | V | NUM-*, UI (Ctrl+F) |

## UI — Interaction

| ID | Requirement | Evidence | Conf | Tests |
|---|---|---|---|---|
| UI-001 | Window layout, header (mode menu, undo/redo, main menu), mode default widths; programming display style. | math-window source+blp | V | UI-T008 |
| UI-002 | Menus: mode menu (6 modes + accelerators), primary menu per `06` §3; number-format radio; About content; Close-All confirm for multi-window. | math-window.blp; gnome-calculator.vala | V | STATE-T003 |
| UI-003 | Keypad inventories and behavior per `06` §5 (basic/advanced/financial/programming/conversion/keyboard), inverse modifier, angle-unit cycler Deg→Rad→Grad. | buttons-*.vala/.blp extraction | V | UI-T008..011 |
| UI-004 | Programming specifics: bit panel (64 bits, float exponent highlighting, disabled beyond word size), base label in info line (bases ≠ current, LTR mark, two's-complement magnitude for negatives), per-base exponent button targets, character dialog (3 modes). | buttons-programming source | V | UI-T009/010/020 |
| UI-005 | Display: editable text, answer region (bold; digit-after-result clears; operator continues), status line error styling, spinner >100ms, "Calculating". | math-display/math-equation source | V | UI-T001..004 |
| UI-006 | Keyboard input handling per `06` §9 (substitutions, sup/sub digits, Ctrl-shortcuts, keypad numlock-off, hex letters, Escape/Ctrl+Delete clear). | math-display source; shortcuts blp | V | UI-T006, EXPR-T023 |
| UI-007 | Converter panel per `06` §6 (two synced fields, "Error" propagation, π entries, DMS field format, favorites/currency display modes, offline pane, rate details dialog). | math-converter source | V | UNIT-T*, UI (conversion) |
| UI-008 | Popovers: variables (insert/store/delete, reserved names, constants submenus), functions (categories, add/edit/delete flows). | popover sources | V | UI-T017, UI-T018 |
| UI-009 | Preferences dialog controls per `06` §8; live application; number-format via menu only. | math-preferences source | V | UI-T016 |
| UI-010 | Autocompletion providers, priorities, settings gating, activation behavior. | math-display source | S | (GUI only) |
| UI-011 | History view: entries (equation, answer), click actions, Alt+Left/Right navigation with in-progress memory, re-render on serializer change, restored entries keep stored strings. | math-history/math-window source | V | UI-T013, STATE-T001 |
| UI-012 | Clear semantics: C/Escape = display; Clear History menu/Ctrl+Escape = history+tape. | math-window source; accelerators | V | UI-T006/007, STATE-T002 |
| UI-013 | Undo/redo: state stack semantics per `05` §7; accelerators; enabled state buttons. | math-equation source | V | UI-T012 |
| UI-014 | Accessibility: announcements (status, result), a11y invalid state on name entries, LTR enforcement, shortcuts window. | sources; NEWS | V | (a11y audit) |

## FMT — Formatting

| ID | Requirement | Evidence | Conf | Tests |
|---|---|---|---|---|
| FMT-001 | Four formats with exact rendering rules per `07` §2 (rounding half-away at digit A+1, thresholds 64/22/16/12, base-group counts 3/4). | serializer source; tests | V | FMT-T001..012 |
| FMT-002 | Base display: grouping non-decimal, representation-base suffixes, exponent in current base with hex superscripts. | serializer; NEWS; tests | V | FMT-T003..009, UNIT-T019 |
| FMT-003 | Complex formatting rules (i display, sign suppression). | serializer; tests | V | FMT-T016, FUNC-T019 |
| FMT-004 | Sign/zero: U+2212 minus; negative zero normalized; trailing zero toggle. | serializer; tests | V | NUM-T018, FMT-T015 |
| FMT-005 | Locale radix/tsep in input and output across GUI/CLI/search; tsep="." guard. | sources | V | LOC-T001..003 |
| FMT-006 | DMS display with carry; DMS input. | serializer; lexer | V | LIT-T010..012 |
| FMT-007 | Fixed-format cap 2^precision with error message. | gnome-calculator.vala; serializer | V | NUM-T023 |
| FMT-008 | Copy strips separators; paste newlines→spaces. | math-equation source | V | UI-T014/015 |

## STATE — Persistence & integration

| ID | Requirement | Evidence | Conf | Tests |
|---|---|---|---|---|
| STATE-001 | GSettings keys per `08` §1 with exact defaults and write moments (incl. dead `window-position`). | gschema; sources | V | STATE-T003/006 |
| STATE-002 | History tape persistence: written on window close, cleared by Clear History, restored at startup. | math-window source; gschema | V | STATE-T001/002 |
| STATE-003 | Variable/function persistence files (registers, custom-functions) with formats. | sources | V | STATE-T004/005 |
| STATE-004 | CLI `--solve` semantics: normalization, base10/wl32/angle-from-settings, UNKNOWN_RATE retry, output/error/exit codes. | gnome-calculator.vala | V | CLI-T001..004 |
| STATE-005 | `--equation`, `--mode`, `--version` behaviors. | gnome-calculator.vala | V | CLI-T005 |
| STATE-006 | gcalccmd binary: arg mode + REPL, always-degrees, error formats. | gcalccmd.vala | V | CLI-T006/007 |
| STATE-007 | Search provider contract: single result, rejections (plain number, `=`, budget), metadata, activation copy + notification, Open-in-Calculator spawn, 10-entry cache, 10s idle timeout. | search-provider.vala | V | SP-T001..005 |
| STATE-008 | Currency caches under XDG cache; exchange-api.gnome.org URLs; file staleness logic. | provider sources | V | STATE-T007 |
| STATE-009 | Multi-window/multi-instance behavior; shutdown writes from last window; per-window history save. | gnome-calculator.vala | S | STATE-T006 |

## COMPAT — Cross-cutting compatibility

| ID | Requirement | Evidence | Conf | Tests |
|---|---|---|---|---|
| COMPAT-001 | All upstream engine test assertions (`09` §1–7) reproduce exactly at C locale. | tests | V | all |
| COMPAT-002 | GUI test scenarios (`09` §8) reproduce at the UI layer (with corrected error strings per `05` §4 — upstream dogtail strings are stale, OQ-01). | dogtail + analysis | S | UI-T* |
| COMPAT-003 | CLI behaviors (`09` §9) byte-exact on stdout, message-exact on stderr, exit codes exact. | sources | V | CLI-T* |
| COMPAT-004 | Search provider DBus contract equivalent (results/metadata/activation). | search-provider.vala | S | SP-T* |
| COMPAT-005 | Persistence: history/variables/functions survive restart with identical content. | sources | V | STATE-T* |

## Traceability matrix (feature → requirements → tests)

| Feature (`01` §3) | Requirements | Key tests |
|---|---|---|
| Expression entry & evaluation | EXPR-001..016, NUM-001..014 | EXPR-*, NUM-*, UI-T001..005 |
| Basic arithmetic | EXPR-006, NUM-001..003 | EXPR-T001..040 |
| Scientific functions | NUM-006..008, FUNC-001 | FUNC-T* |
| Constants & variables | FUNC-002..004 | FUNC-T017, VAR-T* |
| User functions | FUNC-003 | VAR-T007..013 |
| Unit conversion | FUNC-005/006, EXPR-012 | UNIT-T001..018 |
| Currency | FUNC-007 | UNIT-T022/023, STATE-T007 |
| Programming mode | EXPR-010/011, NUM-011, UI-004 | NUM-T024..028, UI-T009..011 |
| Financial mode | FUNC-008 | UI-T019 |
| History | UI-011, UI-012, STATE-002 | UI-T006/007/013, STATE-T001/002 |
| Formatting & localization | FMT-001..008 | FMT-*, LOC-* |
| Preferences & state | UI-009, STATE-001 | UI-T016, STATE-T003/006 |
| CLI / search provider | STATE-004..007 | CLI-*, SP-* |
| Accessibility | UI-014 | a11y audit (manual) |

**Unresolved dependencies/ambiguities:** see `11` (each OQ references the requirements above).
