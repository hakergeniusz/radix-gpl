# 12 — Final Status Report (Team A Handoff)

**Date:** 2026-10-30 · **Upstream analyzed:** GNOME Calculator `master @ bf92c82b` (2026-10-30), v51.1 · **Method:** static source analysis + upstream test extraction + help/NEWS corroboration. The original application was **not executed** (see OQ-18).

## 1. Documents completed

All twelve deliverables from PROMPT.md §5 exist in `docs/`:

| Doc | Content | State |
|---|---|---|
| 00 | Project overview, scope, modes, constraints | Complete |
| 01 | Repository/feature inventory, subsystem map, glossary, feature matrix | Complete |
| 02 | Expression language spec (tokens, literals, grammar, precedence, conversions, assignments, errors) | Complete |
| 03 | Numeric semantics (precision, exact rationals, complex, per-operation domains and edge cases) | Complete |
| 04 | Catalogs: 40+ built-in functions, 21 constants, 12+2 unit categories (~150 units, exact formulas), 67 currencies + 4 providers, 10 financial functions | Complete |
| 05 | Evaluation lifecycle, error surfacing (full message table), limits, undo/redo/history interplay | Complete |
| 06 | UI/interaction: all 6 modes, complete keypad button inventories (extracted from UI definitions), menus, shortcuts, popovers, preferences, converter, state transitions | Complete |
| 07 | Formatting rules (4 formats with exact algorithms), localization, RTL, accessibility, clipboard | Complete |
| 08 | GSettings matrix (21 keys, write moments), persistence files, CLI (--solve/gcalccmd), search provider contract | Complete |
| 09 | Compatibility test catalog: ~150 catalogued cases across 11 groups with IDs/priorities/evidence, coverage gaps | Complete |
| 10 | 75 requirements (EXPR/NUM/FUNC/UI/FMT/STATE/COMPAT) with evidence, confidence, test links + traceability matrix | Complete |
| 11 | 18 open questions with resolution paths; 5 resolved contradictions documented | Complete |
| INDEX | Navigation + status | Updated |

## 2. Coverage assessment

**Fully analyzed (source-read, spec-written, test-linked):** engine (lexer, parser, number, serializer, evaluation orchestration, budget), functions/constants/variables, units, currency + providers, financial, formatting, settings/persistence, CLI, search provider, keypad UI (complete button-level extraction), menus/shortcuts/preferences, engine test suites, help corpus, NEWS deltas v46→51.1.

**Analyzed but with reduced confidence (inferred/strongly-supported only):** completion popup UX timing (OQ-17), keyboard-mode OSK specifics (OQ-11), undo/solve deep interaction sequences (OQ-08), per-locale `in`-keyword coverage (OQ-09).

**Not analyzed (out of scope, documented as such):** GCalc/GCI parallel libraries (`01` §1), visual design/CSS details beyond behavior-relevant classes, packaging scripts beyond metadata facts, `po/` per-locale content (mechanism only).

## 3. Requirement & test coverage

75 requirements; every requirement carries ≥ 1 linked test or an explicit reason a test cannot yet run (GUI-only cases in `09` §8). Test catalog: ~150 cases, priorities assigned, ~380 upstream assertions available as ground truth for engine behavior. Known un-testable statically: currency provider live formats (OQ-06), multi-window tape semantics (OQ-07).

## 4. Major compatibility risks for Team B (ranked)

1. **Exact rational arithmetic** (NUM-002): the v51 rational shadow changes user-visible results (`0.1+0.2 = 0.3`, 19-digit cancellation). Reproducing with naive binary floats is impossible; arbitrary-precision rationals are mandatory.
2. **Percentage sugar + implicit multiply precedence** (EXPR-006/007): `4÷2(1+1)=4` and the `a±b%` quirks are the most likely silent incompatibilities.
3. **Formatting algorithms** (FMT-001/002): display rounding half-away-from-zero at digit A+1, automatic thresholds per base (64/22/16/12), grouping counts 3/4, exponent-in-current-base — all byte-exact expectations in tests.
4. **Precision model** (NUM-001): 2000-bit default + fixed-format cap 2^precision error path.
5. **Error message strings** (EXPR-015): exact translated English strings specified; Python-style “%s” quoting (curly quotes) matters for UI parity.
6. **Search provider/CLI contracts** (STATE-004..007): output formats and exit codes are externally observable.

## 5. Unresolved questions

18 open questions in `11`; none block implementation start. The three with real behavioral impact: OQ-02 (CLI precision 1000 vs 2000), OQ-04 (untested percentage-sugar combinations), OQ-06 (live currency file formats).

## 6. Evidence limitations

- No runtime verification (OQ-18): all "Verified" claims are source/test-derived. A runtime confirmation pass is recommended before feature-freeze, prioritizing: locale radix/grouping behavior, currency refresh flow, search provider results, a11y announcements, completion UX.
- Dogtail GUI expectations contain stale error strings (OQ-01); `09` §8 uses source-verified messages instead.
- Currency rate files drift upstream (GNOME proxy mitigates; provider parsers are the stable contract).

## 7. Recommended next steps

1. Team B bootstrap against `00`–`05` (engine first, headless), using `09` §1–7 as acceptance gates.
2. Runtime verification pass on upstream (build + exercise) to close OQ-02/03/07/17 — Team A can then upgrade S→V claims.
3. GUI-layer acceptance tests for `09` §8 once a toolkit is chosen.
4. Keep `INDEX.md` statuses updated as documents evolve; requirement IDs are stable — extend, never renumber.

## 8. Clean-room boundary statement

Team A read the original repository (checkout at `upstream/gnome-calculator/`, read-only, unmodified) and produced only original prose, tables, and independently verified numerical examples. No source code, code fragments, or structural blueprints were copied into the deliverables. Source file/symbol references are confined to `01`, `10` (traceability), and `11` for audit purposes; the implementation-facing documents `02`–`09` are implementation-independent. Licensing: upstream is GPL-3.0-or-later; the specification is behavioral documentation — the project owner should confirm the licensing approach for Radix (behavioral specs are generally not copyrightable expression, but this review is outside Team A's mandate).
