---
name: stdlib-docs
description: Regenerate the Kestrel stdlib reference. Runs `kestrel-doc` against `lang/std`, writes JSON to `external/kestrel-website/public/stdlib/` (consumed by the website) and markdown to `docs/stdlib/` (consumed by Context7). Use when the user says "regenerate stdlib docs", "update the stdlib reference", "rebuild the docs", or after editing a `.ks` file under `lang/std/` whose `///` doc comments changed.
---

# stdlib-docs

Two outputs, one source of truth:
- **JSON** at `external/kestrel-website/public/stdlib/*.json` — powers the docs site (`external/kestrel-website/src/pages/StdlibItem.tsx` reads these).
- **Markdown** at `docs/stdlib/*.md` — indexed by Context7 via the repo-root `context7.json`.

Both come from the same `kestrel-doc` walk over `lang/std`. Always regenerate both together so the two surfaces stay in sync.

## Default move

```bash
cargo run -p kestrel-doc -- \
  --src lang/std \
  --out external/kestrel-website/public/stdlib \
  --md-out docs/stdlib \
  --format both
```

## When to run

- After editing `///` doc comments on any public stdlib item.
- After adding/removing a public type, function, protocol, extension, init, or subscript in `lang/std/`.
- After renaming a module file under `lang/std/` (changes the module path → changes filenames).
- Before committing changes that touch `lang/std/` if the diff already shows stale `external/kestrel-website/public/stdlib/*.json` modifications.

Skip for purely internal (non-`public`) stdlib edits — they don't affect the rendered docs.

## After running

1. Check `git status` — both `external/kestrel-website/public/stdlib/` and `docs/stdlib/` should show changes.
2. Skim one or two changed `.md` files to confirm the formatting renders sensibly (signatures fenced, `# Examples` demoted under item heading).
3. Commit both trees together. They're a coupled pair; never commit one without the other.

## Variants

- **JSON only** (rare; only when iterating on the website): `--format json`.
- **Markdown only** (rare; only when iterating on Context7 ingestion): `--format markdown`.
- **Different source tree** (e.g. an ecosystem library that wants Context7 docs): `--src <path>`. The same CLI works on any Kestrel source tree.

## Implementation pointers (don't touch unless changing emitter behavior)

- Walk + extraction: `lib/kestrel-doc/src/lib.rs` (`extract` returns `(ModuleIndex, Vec<ModulePage>)`).
- JSON shape: `Item`, `MemberGroup`, `ModulePage` in `lib.rs`.
- Markdown printer: `lib/kestrel-doc/src/markdown.rs` (`render(&ModulePage) -> String`).
- Signature builder: `lib/kestrel-doc/src/signature.rs`.

If a stdlib item renders badly in markdown (e.g. heading nesting wrong, signature not fenced, `@name` directive leaking through), fix `markdown.rs`, not the source `.ks` file.

## Context7 wiring

`context7.json` at the repo root scopes Context7 to `docs/stdlib/` and `docs/language/`. Touching that file is rare — only when adding new top-level doc folders or adjusting `excludeFolders`.
