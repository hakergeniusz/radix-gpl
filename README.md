# Radix

Radix is a Rust reimplementation of the GNOME Calculator's behavior
(target: upstream v51.1 semantics), built from the behavioral specification in
[`docs/`](docs/INDEX.md). It provides a
GTK4 desktop calculator, a console calculator, a headless solve CLI, and a
GNOME Shell search provider.

License: GPL-3.0-or-later (see `LICENSE`; `IMPLEMENTATION_DECISIONS.md` D34
records the relicensing to GPL-3.0-or-later and the `rug` dependency
rationale).

## Build

```sh
cargo build --workspace          # debug binaries in target/debug/
cargo build --release            # release profile: thin LTO
```

Requirements: a Rust 2021 toolchain, GMP/MPFR/MPC (`rug`), and GTK4 +
libadwaita (the GUI crates link them; `radix-core` alone needs neither).

## Binaries

| Binary | Crate | Purpose |
|---|---|---|
| `radix` | radix-gui | The desktop calculator (GTK4). Also answers the `gnome-calculator`-style CLI contract. |
| `radix-cmd` | radix-cli | Console calculator with a `> ` REPL (the `gcalccmd` contract). |
| `radix-search-provider` | radix-gui | GNOME Shell search provider on the session bus. |

### `radix` CLI

```sh
radix --solve "2+2"          # prints 4, exit 0; errors on stderr, exit 1
radix --solve "10 in hex"    # A₁₆ (base conversions override the display base)
radix --equation "9*8"       # opens a window with the equation loaded (\n etc. unescaped)
radix -m programming         # start mode: basic|advanced|financial|programming|keyboard|conversion
radix --version              # "Radix 51.1.0" on stderr, exit 0
```

Evaluation context for `--solve` (per doc 08 §4): base 10, word length 32,
precision 1000, AUTOMATIC format at accuracy 9, C locale. The GUI evaluates at
precision 2000; long transcendental computations may differ in the last digits
between the two (documented in the spec).

### `radix-cmd`

```sh
radix-cmd "2+3"    # one-shot: prints 5
radix-cmd          # REPL: `> ` prompt; `exit`, `quit`, empty line, or EOF exits
```

### Search provider

`radix-search-provider` owns `org.gnome.Radix.SearchProvider` and serves
`org.gnome.Shell.SearchProvider2` at `/org/gnome/Radix/SearchProvider`. A
D-Bus service file (`crates/radix-gui/data/dbus-1/services/`) with an
`@bindir@` placeholder ships for install-time substitution. Shell queries
evaluate exactly like `--solve` (50 ms budget); activating a result copies it
and shows a "Result copied" notification; the service idles out after 10 s.

## Tests

```sh
cargo test --workspace      # 181 tests, no display or network required
```

- `radix-core` tests map one-to-one onto the compatibility catalog in
  `docs/09` §1–7 (expression parsing, literals, numerics, functions,
  variables, units, serialization) plus persistence, settings, currency, and
  financial formula tests.
- `crates/radix-gui/tests/cli.rs` and `crates/radix-cli/tests/cmd.rs` check
  the CLI contracts byte-exactly (doc 09 §9 CLI-T001..007). They run the
  built binaries with XDG dirs redirected to a scratch dir.

GUI behaviors (doc 09 §8) are runtime-verified in isolated sessions —
Xvfb display + private D-Bus (`dbus-run-session`), never on a desktop
session; see HANDOFF.md for the harness recipe.

## Layout

```
crates/radix-core        engine: lexer, parser, eval, number (rug), serializer,
                         units, currency, financial, persistence, settings
crates/radix-cli         radix-cmd binary + shared CLI plumbing
crates/radix-gui         GTK4 app (window/keypads/converter/dialogs), the
                         headless --solve path, and the search provider
docs/                    the specification this implementation follows
REQUIREMENT_COVERAGE.md  requirement-by-requirement status with evidence
IMPLEMENTATION_DECISIONS.md   decision log D1–D21 (ambiguity resolutions, deviations)
HANDOFF.md               architecture, verification playbook, known gaps
```
