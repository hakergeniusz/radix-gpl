# Translations (gettext skeleton)

Radix is localized with plain gettext. The **msgid is the exact English
UI string** — the clean-room spec text — so an untranslated or missing
catalog renders the spec English unchanged. The infrastructure is
complete; what is missing are the per-language catalogs. This file is
the whole workflow.

## Files

| Path | Purpose |
|---|---|
| `radix.pot` | Template with every translatable string (generated) |
| `<lang>.po` | One language's translations (add these) |
| `<lang>/LC_MESSAGES/radix.mo` | Compiled catalog, committed next to the .po |

The runtime binds domain `radix` to the first of: `$RADIX_LOCALEDIR`,
`/app/share/locale` (Flatpak), `/usr/share/locale` (deb/RPM/Arch),
`crates/radix-gui/data/locales/` (dev tree, debug builds). A dir is only
used if it actually contains a `radix.mo`, so an empty tree means
English.

## Adding a language `<lang>` (e.g. `de`, `pl`)

1. Copy the template: `cp radix.pot <lang>.po`.
2. Fill the header: `Language: <lang>`, correct `Plural-Forms` for the
   language (see gettext's language list), and `Project-Id-Version`.
3. Translate every `msgstr`. The msgid is English UI text; translate
   meaning, not words. Strings with `{}`/`%s`-like placeholders keep
   them (there are currently none — no formatted strings are
   translated).
4. Compile and place it (the .mo is committed too — packages install the
   tree verbatim, no msgfmt at package build time):
   ```sh
   msgfmt -o <lang>/LC_MESSAGES/radix.mo <lang>.po
   ```
5. Check it in the dev tree (debug build picks up this directory):
   ```sh
   LANGUAGE=<lang> cargo run -p radix-gui
   ```
6. Commit both `<lang>.po` and `<lang>/LC_MESSAGES/radix.mo`.

## What is translatable, and what is not

Collected into the POT (see `tools/extract-pot.py` for the sources):

- All wrapped GUI literals: mode names, menus, tooltips, preferences,
  financial/character dialog labels, shortcuts window, status lines.
- Unit **display names** and **category display names** (the dropdowns
  show translated labels; the parser keys stay English).
- Function **descriptions** in the f(x) popover.

Deliberately NOT translatable (never translate these):

- Button glyph labels (√, x², sin, mod, ≪ …) — symbols.
- Function names (`sqrt`, `sin` …), variable names, currency codes.
- Unit parser symbols (`degree`, `deg`, …) and unit keys.
- Engine error messages — spec-pinned English (doc 03/04); the tests
  assert them.
- The product/window name "Calculator".

## Regenerating the POT

After any UI change:

```sh
python3 tools/extract-pot.py
git add crates/radix-gui/data/locales/radix.pot
```

Then `msgmerge` existing .po files against the new POT
(`msgmerge --update <lang>.po radix.pot`) and recompile their .mo.
