# Radix — Team A Documentation Index

Clean-room behavioral specification of GNOME Calculator (upstream `master @ bf92c82b`, 2026-10-30, v51.1) for the Rust replacement **Radix**.

**Team B should read these documents only.** Source-code references are concentrated here, in `01`, in the traceability matrix in `10`, and in the audit notes in `11`; the specification documents `02`–`09` are implementation-independent.

| # | Document | Content | Status |
|---|---|---|---|
| 00 | [Project overview](00-project-overview.md) | Purpose, workflows, modes, scope, constraints | Complete |
| 01 | [Repository & feature inventory](01-repository-and-feature-inventory.md) | Subsystems, feature matrix, glossary, traceability map | Complete |
| 02 | [Expression language spec](02-expression-language-spec.md) | Literals, operators, precedence, grammar, conversions, errors | Complete |
| 03 | [Numeric semantics](03-numeric-semantics.md) | Precision, exact rationals, complex, per-operation domains, edge cases | Complete |
| 04 | [Functions, constants & units](04-functions-constants-and-units.md) | Complete catalogs (functions, constants, units, currencies, financial) | Complete |
| 05 | [Evaluation & error semantics](05-evaluation-and-error-semantics.md) | Lifecycle, error surfacing, limits, undo/redo/history interplay | Complete |
| 06 | [UI & interaction spec](06-ui-and-interaction-spec.md) | Screens, keypads (full button inventories), menus, shortcuts, dialogs | Complete |
| 07 | [Formatting, localization & accessibility](07-formatting-localization-and-accessibility.md) | Display algorithms, locales, RTL, a11y, clipboard | Complete |
| 08 | [State, persistence & integration](08-state-persistence-and-integration.md) | Settings matrix, files, CLI, gcalccmd, search provider | Complete |
| 09 | [Compatibility test catalog](09-compatibility-test-catalog.md) | ~150 prioritized test cases with IDs and evidence | Complete |
| 10 | [Requirements & traceability](10-requirements-and-traceability.md) | 75 requirements with evidence/confidence + matrix | Complete |
| 11 | [Open questions & uncertainties](11-open-questions-and-uncertainties.md) | 18 open questions + resolved contradictions | Complete |
| 12 | [Final status report](12-final-status-report.md) | Coverage, risks, recommendations, clean-room statement | Complete |
| 13 | [Visual design spec](13-visual-design-spec.md) | Looks: geometry, colors, typography, keypad styling, icons, responsive collapse; dark reference screenshot | Complete |

Related workspace paths:

- `upstream/gnome-calculator/` — original source checkout (Team A only; evidence base for citations).
- `assets/reference-basic-dark.png` — user-provided screenshot of the original (dark, basic mode), normative for doc 13's measured values.
- Evidence/confidence vocabulary: `10` §0.
- Suggested Team B reading order: 00 → 01 → 02 → 03 → 05 → 04 → 07 → 09 (engine first), then 06/08 for the app layer, then 13 for the visual layer.
