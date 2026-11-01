# Radix — Implementation Plan (Team B)

**Status:** living document. Spec: `docs/` (Team A, v51.1 target). Created 2026-10-30.

## 1. Guiding constraints

1. Spec-driven: docs 00, 02–09 are the behavioral contract; never read `upstream/` or `notes/`.
2. Behavioral compatibility first; byte-exact display strings and error messages where the catalog demands them.
3. Requirement IDs from doc 10 are preserved; coverage tracked in `REQUIREMENT_COVERAGE.md`.
4. Ambiguities resolved per doc 11 (OQ-*) and recorded in `IMPLEMENTATION_DECISIONS.md`; never silently chosen.
5. Engine is headless and GUI-independent; GUI delegates all math to it.

## 2. Architecture

Cargo workspace at `~/Projects/radix/`:

| Crate | Kind | Contents |
|---|---|---|
| `crates/radix-core` | lib | Numeric engine, lexer, parser, evaluator, serializer, units, currency, financial, settings/persistence, evaluation budget, top-level solve pipeline |
| `crates/radix-cli` | bins `radix`, `radix-cmd` | `--solve`/`--equation`/`--mode`/`--version` (doc 08 §4) and the gcalccmd-style REPL (doc 08 §5) |
| `crates/radix-gui` | bin `radix-gui` | GTK4/libadwaita frontend (doc 06): modes, keypads, converter, history, popovers, preferences, bit panel |

`radix-core` module map (conceptual areas of the task brief → modules):

| Area | Module(s) | Spec |
|---|---|---|
| Numeric representation & arithmetic | `number` (Number = complex re/im at precision P + exact `Rational` shadow) | 03 |
| Expression lexing | `lexer` | 02 §2–4 |
| Parsing | `parser`, `ast` | 02 §5–13 |
| Evaluation | `eval`, `functions`, `constants`, `variables` | 03 §4, 04 §1–2 |
| Unit conversion | `units` | 04 §3.1 |
| Currency | `currency` (+ HTTP providers) | 04 §3.2, 08 |
| Financial | `financial` | 04 §4 |
| Formatting | `serializer` | 07 |
| Errors | `error` (ErrorCode + exact English messages) | 02 §10, 05 §4 |
| State/persistence | `settings`, `history` | 08 §1–2 |
| Evaluation limits | `budget` | 05 §5 |
| Solve pipeline (canonicalization, assignment, factorize) | `solve` | 02 §11, 05 |

## 3. Dependencies (independently evaluated)

| Crate | Purpose | Why |
|---|---|---|
| `rug` (GMP/MPFR/MPC) | Arbitrary-precision floats/complex + exact rationals | Doc 03 semantics *are* MPFR/MPC/mpq observable behavior (2000-bit round-to-nearest, `mpfr_frac` trunc semantics, MPC branches). A pure-Rust re-implementation would re-derive correctly-rounded transcendentals — high risk, no fidelity gain. System libs present. Licensing: LGPL libraries, Radix ships GPL-3.0-or-later. |
| `gtk4` + `libadwaita` | GUI | Doc 06 toolkit-independence permits any toolkit; native GTK4 matches platform expectations. |
| `ureq` (rustls) | Currency provider downloads | Small sync client; network use limited to currency rates (doc 04 §3.2). |
| `quick-xml` | ECB/BOC XML rate parsing | Provider file formats are XML. |
| `zbus` | Search-provider DBus service | Pure-Rust org.gnome.Shell.SearchProvider2 (doc 08 §6). |
| `getrandom`/std | `rand` variable | Uniform [0,1) per read (doc 03 §7). |

Full dependency records (version, license, reason) are maintained in `IMPLEMENTATION_DECISIONS.md` §Deps.

## 4. Milestones / order (maps to task §7 phases)

| M | Phase | Deliverable | Gate |
|---|---|---|---|
| M0 | 0 | This plan + coverage matrix + decision log | done |
| M1 | 1–2 | Workspace builds; `number` engine + rational shadow + edge cases; unit tests for doc 03 | NUM-T001..030 (subset without parser) |
| M2 | 3 | Lexer + parser + AST + precedence quirks | EXPR-T001..041, LIT-T001..018 |
| M3 | 4 | Evaluator: functions/constants/units/currency/financial | FUNC-T*, VAR-T*, UNIT-T001..018 |
| M4 | 5 | Serializer (4 formats, bases, complex, DMS) | FMT-T001..017, UNIT-T019..021, LIT-T016 |
| M5 | 6 | Settings, registers, custom functions, history tape | STATE-T004/005 |
| M6 | 8a | CLI binaries + exit codes | CLI-T001..007 |
| M7 | 7 | GTK4 GUI (all 6 modes, keypads, converter, history, popovers, prefs, bit panel) | UI-T* (manual/automated) |
| M8 | 8b | Search provider (zbus) with budget | SP-T001..004 |
| M9 | 9 | Full-catalog audit run, fix issues | doc 09 §1–7 green; gaps recorded |
| M10 | 10 | Release readiness: fmt/clippy clean, README/run instructions, handoff report | task §13 checklist |

Engine-first ordering rationale: GUI must delegate to a numerically correct engine; serializer depends on number; CLI is nearly free once the engine exists and validates the full pipeline headlessly.

## 5. Testing strategy

- Doc 09 §1–7 → Rust unit/integration tests in `radix-core` (exact-string expectations, C locale, accuracy 9).
- Doc 09 §9 → integration tests spawning the CLI binaries (stdout/stderr/exit codes).
- Doc 09 §8 → GUI-layer checks; automated where feasible, otherwise manual acceptance list in the handoff report.
- Doc 09 §11 gaps (no upstream test) → tests marked `[SPEC]`/`[INFERRED]` in test names.
- `cargo test` must pass headless; GUI crate keeps its own lighter suite (state transitions without launching windows where possible).
- Never weaken a test to pass; mismatches become decision-log entries with evidence.

## 6. High-risk areas → mitigations (task §4)

| Risk | Mitigation |
|---|---|
| A. Exact rationals | rug `Rational` shadow carried on Number; dropped on inexact ops; exactness tests NUM-T001..005, T018, T022 |
| B. Precedence quirks | Recursive-descent parser built strictly from doc 02 §5 table; every row of the table has a test |
| C. Byte-exact formatting | Serializer works on digit strings from correctly-rounded conversions; doc 07 algorithms implemented literally; FMT-T* exact strings |
| D. 2^2000 fixed cap | Magnitude check against 2^P before fixed/automatic rendering; boundary tests both sides |
| E. CLI 1000-bit vs GUI 2000-bit | Precision is an explicit parameter of the evaluation context; CLI contexts use 1000, GUI 2000 (OQ-02 documented) |
| F. Stale error strings | Doc 05 §4 messages only; dogtail strings ignored (OQ-01) |

## 7. Progress reporting

Status updates per milestone in this file §8 (below) + final handoff report at `HANDOFF.md` (M10).
