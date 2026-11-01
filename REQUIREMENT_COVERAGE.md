# Radix — Requirement Coverage Matrix

Statuses: `NOT_STARTED` / `IN_PROGRESS` / `IMPLEMENTED` (code exists) / `TESTED` (passing verification) / `BLOCKED` / `OUT_OF_SCOPE`.
Requirement IDs and wording are Team A's (doc 10), extended never renumbered. Descriptions abbreviated — read doc 10 for the full contract.

Verification evidence as of the Phase 9 audit + doc 13 visual pass (2026-10-30):
- **Core automated**: `cargo test --workspace` — 181 tests green across 6 suites (radix-core lib 7, numeric 60, serializer 27, solve-catalog 78, `radix` CLI 5, `radix-cmd` 4). The solve-catalog suite covers doc 09 §1–6 test-by-test (EXPR/LIT/NUM/FUNC/VAR/UNIT-T*).
- **GUI runtime-verified**: G1–G3 sessions on an isolated Xvfb + private D-Bus (never the user's session): window shell, all keypads, converter, bit panel, menus/preferences, accelerators, dialogs, copy/paste, history, undo. Plus the **doc 13 visual audit** (staged screenshot pass, dark style): basic reference reproduction (history "2+2 = 4" / "9+10" caret), advanced wide + narrow carousel, programming bit panel, converter, financial Ctrm dialog + result (7.272540897, back button).
- **Search provider runtime-verified**: SP-T001..005 on an isolated session bus (metas, rejections, clipboard, notification + 4 s withdraw, LaunchSearch window, 10 s idle exit).
- **CLI automated**: doc 09 §9 CLI-T001..007 byte-exact subprocess tests (`crates/radix-gui/tests/cli.rs`, `crates/radix-cli/tests/cmd.rs`).

| ID | Summary | Location | Tests | Status | Notes |
|---|---|---|---|---|---|
| EXPR-001 | Tokens incl. Unicode variants, whitespace, unknown chars error | core/lexer | EXPR-T031..041 | TESTED | solve-catalog suite |
| EXPR-002 | Any Unicode Nd digits; A–F hex; fullwidth hex | core/lexer | LIT-T013/014 | TESTED | Arabic-Indic + Persian digits |
| EXPR-003 | Base suffixes, prefixes, agreement, digit-range errors | core/lexer+parser | LIT-T001..008 | TESTED | incl. LIT-T008 >64-bit via suffix |
| EXPR-004 | Vulgar fractions, DMS literals | core/lexer | LIT-T009..012,015 | TESTED | |
| EXPR-005 | Word classification order; multi-char var products; localized in/to | core/lexer+parser | VAR-T001..004, UNIT-T016..018 | TESTED | D14: `in`/`to` via string catalog |
| EXPR-006 | Precedence/associativity table incl. implicit multiply, right-assoc ^ | core/parser | EXPR-T001..014 | TESTED | |
| EXPR-007 | Percentage postfix + additive sugar quirks | core/parser+eval | EXPR-T015..022 | TESTED | D6 |
| EXPR-008 | Superscript powers, nth roots, function superscripts | core/parser | EXPR-T023..028, FUNC-T005 | TESTED | |
| EXPR-009 | Grouping (), [] round, {} frac-part, ⌊⌋ ⌈⌉ ‖ | core/parser | NUM-T013..015, EXPR-T004 | TESTED | |
| EXPR-010 | Bitwise nibble semantics, masking rules | core/number | NUM-T024..026 | TESTED | |
| EXPR-011 | Shifts mod word size, sign rules, overflow | core/number | NUM-T027/028 | TESTED | |
| EXPR-012 | Conversions: units require from-unit; base conversion EOL-only; currency | core/parser+eval | UNIT-T001..024 | TESTED | |
| EXPR-013 | Assignment & function definition rules | core/parser+eval | VAR-T005..013 | TESTED | |
| EXPR-014 | Canonicalization display↔parser | core/solve + GUI | UI-T014/015 | TESTED | GUI runtime-verified (G2) |
| EXPR-015 | Error classes + exact messages | core/error | CLI-T002/003, UI-T002 | TESTED | automated CLI tests assert bytes |
| EXPR-016 | Partial input never displayed; commit evaluates | core/solve + GUI | UI-T001..004 | TESTED | GUI runtime-verified (G1) |
| NUM-001 | MPFR/MPC complex at P bits (GUI 2000) | core/number | NUM-T022 | TESTED | D5 precision split |
| NUM-002 | Exact rational shadow | core/number | NUM-T001..005 | TESTED | |
| NUM-003 | Division by zero error, never inf | core/number | NUM-T006, UI-T002 | TESTED | |
| NUM-004 | 0^0, 0^neg errors | core/number | EXPR-T029/030 | TESTED | |
| NUM-005 | Power/root branching rules | core/number | FUNC-T015..020 | TESTED | |
| NUM-006 | ln/log domains, complex branches, log_N | core/number | FUNC-T012..014, T022 | TESTED | |
| NUM-007 | Trig exactness guards, domains, angle units | core/number | FUNC-T001..009 | TESTED | |
| NUM-008 | Factorial incl. Γ path, poles | core/number | NUM-T008..011 | TESTED | |
| NUM-009 | mod / modexp semantics | core/number | NUM-T012, NUM-T029 | TESTED | |
| NUM-010 | Rounding family; {} vs frac() | core/number | NUM-T013..015 | TESTED | |
| NUM-011 | Word-size reinterpretation; float bit view | core/number+GUI | UI-T010/011 | TESTED | bit panel runtime-verified (G2) |
| NUM-012 | rand per read; _ answer | core/eval | NUM-T030, VAR-T015 | TESTED | |
| NUM-013 | −0 normalized; +π branches | core/number | NUM-T018, FUNC-T021/22 | TESTED | |
| NUM-014 | ∞/−∞/NaN display | core/serializer | FMT-T017 | TESTED | |
| FUNC-001 | Built-in function catalog | core/functions | FUNC-T*, VAR-T012 | TESTED | financial names deliberately excluded (dialog-only, doc 04 §4) |
| FUNC-002 | Constants catalog, protected, Cyrillic е | core/constants | FUNC-T017, VAR-T006 | TESTED | |
| FUNC-003 | User functions lifecycle | core/variables | VAR-T007..014 | TESTED | VAR-T014 via persistence round-trip tests |
| FUNC-004 | User variables + registers file | core/variables | STATE-T004, UI-T017 | TESTED | registers round-trip tests + variable popover runtime-verified (G2/G3) |
| FUNC-005 | Unit catalog + ambiguity rule | core/units | UNIT-T001..018 | TESTED | |
| FUNC-006 | Temperature offset conversions | core/units | UNIT-T004 | TESTED | |
| FUNC-007 | Currency + providers + refresh semantics | core/currency | UNIT-T022/023 | TESTED | offline contract tested; live network fetch not exercised (sandbox has no network) |
| FUNC-008 | Financial dialogs + formulas | core/financial + GUI | UI-T019 | TESTED | `financial::evaluate` dispatcher (dialog-only per doc 04 §4); formulas + domains tested; Ctrm dialog runtime-verified → 7.272540897; audit fixed Ctrm/Gpm arity + Ddb off-by-one |
| FUNC-009 | Factorize | core/number+solve | factorize tests, Ctrl+F | TESTED | |
| UI-001 | Window layout/header | gui | UI-T008 | TESTED | G1/G2 runtime; flat one-color header (`headerbar_bg_color` = window bg, D25) |
| UI-002 | Menus | gui | STATE-T003 | TESTED | G3 runtime; D25 rebuilt as `gio::Menu` models on actions with app accelerators (mode radio menu + primary menu: New Window/Clear History/Result Format/Preferences/Shortcuts/Help/About) — hamburger buttons fully working, verified in `programming-mode-menu/-hamburger.png` |
| UI-003 | Keypad inventories per mode | gui | UI-T008..011 | TESTED | all 6 modes (G2); reflowed to the GNOME 51 reference screenshots (D23: shared numeric_core, inverse pairs log↔10^x / ln↔e^x, contextual conversion cell; √/ⁿ√ and +/− keyboard-only); programming bottom row = labelled base/word-size up-dropdowns, cells sized by the grid floor (D25) |
| UI-004 | Programming specifics (bit panel, base label, char dialog) | gui | UI-T009/010/020 | TESTED | G2 runtime; D25: bit panel third carousel page below 700 px (wide card form above), base menu popover verified in `programming-base-menu.png` |
| UI-005 | Display behavior (answer region, status, spinner) | gui | UI-T001..004 | TESTED | G1 runtime |
| UI-006 | Keyboard input handling | gui | UI-T006, EXPR-T023 | TESTED | G1/G2 runtime; D25: plain `.` routes through `insert()` like the keypad dot (undo snapshot, fresh-start after an answer) |
| UI-007 | Converter panel | gui | UNIT-T*, conversion UI | TESTED | G2 runtime |
| UI-008 | Popovers | gui | UI-T017/018 | TESTED | G2/G3 runtime |
| UI-009 | Preferences dialog | gui | UI-T016 | TESTED | G3 runtime |
| UI-010 | Autocompletion | gui | GUI only | NOT_STARTED | settings keys exist (`enabled-completions`); popover UI not built — known gap, see HANDOFF |
| UI-011 | History view | gui | UI-T013, STATE-T001 | TESTED | G3 runtime |
| UI-012 | Clear semantics (display vs history) | gui | UI-T006/007 | TESTED | G3 runtime |
| UI-013 | Undo/redo | gui | UI-T012 | TESTED | G3 runtime |
| UI-014 | Accessibility | gui | a11y audit | NOT_STARTED | widgets use standard labelled buttons (GTK default a11y); no dedicated labels/roles/audit pass |
| FMT-001 | Four formats, exact algorithms | core/serializer | FMT-T001..012 | TESTED | |
| FMT-002 | Base display, grouping, base-suffix, exponent-in-base | core/serializer | FMT-T003..009, UNIT-T019 | TESTED | D17; audit fixed repr_base being dropped by all four solve consumers |
| FMT-003 | Complex formatting | core/serializer | FMT-T016, FUNC-T019 | TESTED | |
| FMT-004 | Sign/zero handling | core/serializer | NUM-T018, FMT-T015 | TESTED | |
| FMT-005 | Locale radix/tsep | core/solve+serializer | LOC-T002/003 | TESTED | serializer locale chars tested (`localized_radix_and_thousands`); shipped normalization is C-locale by decision D14 (LOC-T001 full-locale UI parsing deferred) |
| FMT-006 | DMS display/input | core/serializer+lexer | LIT-T010..012 | TESTED | |
| FMT-007 | Fixed cap 2^precision | core/serializer | NUM-T023 | TESTED | |
| FMT-008 | Copy/paste transforms | gui | UI-T014/015 | TESTED | G2 runtime |
| STATE-001 | Settings keys, defaults, write moments | core/settings + gui | STATE-T003/006 | TESTED | D13 keyfile store, all 19 keys; defaults/round-trip/garbage tests; GUI wiring verified (G3) |
| STATE-002 | History tape persistence | core/history + gui | STATE-T001/002 | TESTED | tape in settings keyfile; per-close write verified (G3) |
| STATE-003 | Variable/function persistence files | core/variables | STATE-T004/005 | TESTED | registers + custom-functions round-trip tests under XDG_DATA_HOME/radix |
| STATE-004 | CLI --solve semantics | radix-gui (bin radix) | CLI-T001..004 | TESTED | automated byte-exact: `4\n`, `Error: Division by zero is undefined`, `Error: ErrorCode.INVALID`, `A₁₆` |
| STATE-005 | --equation/--mode/--version | radix-gui | CLI-T005 | TESTED | --version automated (`Radix <ver>` → stderr, exit 0); --equation/-m runtime-verified (G1) |
| STATE-006 | gcalccmd binary contract | radix-cli (bin radix-cmd) | CLI-T006/007 | TESTED | automated: REPL `> 5\n> ` exit 0; EOF clean exit; one-shot arg mode; D19 numeric codes |
| STATE-007 | Search provider contract | radix-search-provider | SP-T001..005 | TESTED | own bus name, SearchProvider2 methods, rejections, metas ` = <result>`, clipboard + 4 s notification withdraw, open-in spawn, 10 s idle exit — verified in isolated session; 10-entry cache code-reviewed (D18) |
| STATE-008 | Currency caches/URLs/staleness | core/currency+currency_fetch | STATE-T007 | TESTED | D16 URLs tested; refresh-interval 0 = cache-only implemented and code-reviewed (no-network sandbox prevents live-fetch test); UNKNOWN_RATE force-refresh retry wired in both CLIs |
| STATE-009 | Multi-window/multi-instance | gui | STATE-T006 | TESTED | --equation second window verified (G1 + LaunchSearch); window-size/maximized persist in keyfile (round-trip tested, save/load code-reviewed); D9 per-window tape, last close wins |
| COMPAT-001 | Upstream engine assertions reproduce | all core tests | doc 09 §1–7 | TESTED | 172 core tests; every §1–7 catalog ID mapped to a named test |
| COMPAT-002 | GUI scenarios reproduce (corrected strings) | gui runtime | doc 09 §8 | TESTED | UI-T001..020 exercised in G1–G3 sessions with doc 05 §4 strings per D11 (stale dogtail strings ignored); not a11y-layer automated |
| COMPAT-003 | CLI byte-exact | cli tests | doc 09 §9 CLI-T001..007 | TESTED | automated subprocess tests, byte-exact stdout/stderr + exit codes |
| COMPAT-004 | Search provider DBus equivalent | sp runtime | doc 09 §9 SP-T001..005 | TESTED | verified on isolated session bus; re-runnable via the harness scripts documented in HANDOFF.md |
| COMPAT-005 | Persistence survives restart | core tests | doc 09 §9 STATE-T001..007 | TESTED | keyfile/registers/custom-functions round-trip tests + GUI close/reopen runtime (G3) |

## Doc 13 visual checklist (§12, audited 2026-10-30, parity round 2 2026-10-31, header/menu + programming round 2026-11-01)

Doc 13 postdates doc 10 and carries no requirement IDs; its §12 checklist is audited here item-by-item instead (staged Xvfb screenshot harness, real libadwaita dark style; the referenced shots are archived under `docs/assets/visual-audit/`, re-taken 2026-10-31 after the D24 layout-parity round and 2026-11-01 after the D25 header/menu + programming round against the GNOME 51 reference screenshots supplied by the prompt owner — see D23–D25: basic reference reproduction (history "2+2 = 4" / "9+10" caret), advanced wide + narrow carousel, programming wide (`programming.png`) with correct history rows, bit panel markers, Decimal/64-bit bottom-row dropdowns, programming narrow carousel (`programming-narrow.png`), mode radio menu with accelerator column, hamburger primary menu, base menu popover, converter (flush card, text-adjacent chevrons, 236 px keypad), financial Ctrm dialog + result (7.272540897, back button); pixel-verified palette: window #222226, card tint ≈#3d3d41 composited over the window, entry #333337, separators rgba(0,0,0,.36), accent #d56199, flat header = window color (D25)). Deviations are D22/D23/D24/D25.

| §12 item | Status | Evidence |
|---|---|---|
| 1. 360/700 defaults, 616 min height, 700 clamp, raised toolbar; close-only header (reference); header = window color (D25) | TESTED | `basic-dark.png` (360 window), `advanced-wide.png` (700 client, card centered, toolbar separation shadow), `programming.png` (flat header, one color with the content); `win.rs` (Clamp 700/700, ToolbarView Raised, `set_size_request(360, 616)`, default heights 360/616, `@define-color headerbar_bg_color @window_bg_color`); `app.rs` `gtk-decoration-layout ":close"` |
| 2. Card margins 12/12/0, radius 12 (programming top-only) + bit panel bottom radius; flush full-width below 700 (D24) | TESTED | `conversion.png`, `programming.png`; CSS `.display-container`, `.display-container.prog`, `.small .display-container` (margin 0 / radius 0 from the breakpoint), `.bit-panel` inside the card |
| 3. History overlay bg + bottom border, bottom-anchored rows, 1.3em tnum, bold answers, dimmed `=`, first-row border hidden; ListBox default bg killed (D24), rows = upstream 7-column homogeneous grid | TESTED | `basic-dark.png` — "2+2 = 4" on the card tint with separators, bold `4`, dimmed `=`; CSS `.history-view list { background: none }`, `.history-view/.history-entry/.dim-eq`; `append_history` grid |
| 4. Entry 1.4em tnum, 14px margins, transparent on card, circular flat backspace; caret visible at startup/after solve (D24) | TESTED | `basic-dark.png` ("9+10" with caret); CSS `.display-view`, circular flat `edit-clear-symbolic` button; `grab_focus` calls |
| 5. Keypad 64×44 cells, 4px spacing, 12px margins; grids fill the viewport (hexpand + homogeneous, D24); number `alpha(currentColor,.2)` 1.2em bolder; operators/`=` 1.6em; `=` accent; x/f(x) down chevrons (reference; D23) | TESTED | `basic-dark.png` + pixel scan: 44 px buttons on 48 px pitch, fills match the reference; CSS `.math-buttons button*` (D22 cell math); pages are direct Stack children so the 236 px minimum reaches the window (conversion window grows to fit; D24) |
| 6. Carousel collapse < 700 (advanced/financial/programming) | TESTED | `advanced-narrow.png` (client 360: 5-col page, page dots), `programming-narrow.png` (bit panel as third carousel page); breakpoint MaxWidth 699 px; `financial.png`/`programming.png` show the wide two-grid form at client 700 |
| 7. Converter 800-weight 181% values, transparent dropdown buttons, chevrons text-adjacent (halign Start, D24), second value +36px gap, pair centered; status page + spinner/link | TESTED | `conversion.png` (0 → 0 pair, unit dropdowns below each value, flush card); currency status page verified in the earlier financial-result session |
| 8. Popovers 9px rows, 28px min-height, bold section labels, up-opening math popovers | TESTED | CSS `.math-popover row/.title-header` + `connect_map` refills; rows/platform hover verified in G2 |
| 9. 18px floating character dialog; financial Cancel/Calculate accent | TESTED | `financial-dialog.png` (Ctrm: Cancel start, accent Calculate end, three entries); `.char-dialog`/`.action-button` CSS |
| 10. Only the three §10 animations | TESTED | Code-reviewed: the stylesheet carries only the 200ms margin transition; scrollbar slide + popover/dialog/carousel/spinner transitions are platform-owned |

## Known gaps (honest leftovers, also in HANDOFF.md)

- **UI-010 Autocompletion** — not implemented; settings keys and the completion-source plumbing exist.
- **UI-014 Accessibility** — no dedicated a11y pass (labels/roles/AT audit).
- **LOC-T001/LOC-T005 beyond C locale** — D14 defers non-C-locale UI localization; serializer/normalization machinery is locale-parameterized and tested.
- **Live network currency fetch** — implemented and exercised only down to the ureq call boundary (no network in the build sandbox); offline/staleness behavior is the tested contract (D16).
- **Dogtail-style a11y automation for §8** — GUI behaviors are runtime-verified manually per session instead (D11 notes the stale upstream strings issue).
