# F2 — LSP rename edits spans that are not the identifier

> `high` · `fragility` · `lib/kestrel-lsp/src/handlers/rename.rs`,
> `lib/kestrel-lsp/src/references.rs`

`textDocument/rename` writes to the user's real files. Three defects, all
reproduced against the compiler in-tree, made it emit `TextEdit`s that either
destroy source or edit the wrong symbol. **Stage 1 (this change) makes the
whole class fail closed** — the server answers "this symbol cannot be renamed"
instead of corrupting the file. Stages 2-4 (below) make these renames actually
work.

## Repro

```kestrel
module Test
func foo(bar: lang.i64) -> lang.i64 { let count = bar; count + bar }
```

Driven through `rename::target_at` → `identifier_for_target` →
`collect_sites` + `push_decl_site` → `build_workspace_edit`, with
`new_name = "renamed"`. The `TextEdit`s below are verbatim output.

## Defect 1 — renaming a `let` local destroys the statement

Cursor on the `count` **use** (`count + bar`).

```
TextEdit { range: 1:38 – 1:54, new_text: "renamed" }   ← `let count = bar;`
TextEdit { range: 1:55 – 1:60, new_text: "renamed" }   ← `count`
```

`1:38 – 1:54` is the whole `let count = bar;` statement. Applying the edit
yields:

```kestrel
func foo(bar: lang.i64) -> lang.i64 { renamed renamed + bar }
```

The binding, its initializer and its `;` are gone.

**Cause.** `HirBody::locals[id].span` for a simple binding is the span of the
`let` *statement*, not the identifier —
`kestrel-hir-lower/src/stmt.rs:126` passes the statement span straight into
`define_local`. `identifier_for_target` (`rename.rs`) returned
`local.span.clone()` verbatim, and because `push_decl_site` tags the site
`RefKind::Direct`, `clip_to_identifier` (`references.rs`) is a documented
no-op for it — nothing narrows the span before it becomes an edit.

## Defect 2 — renaming a parameter splices text at file offset 0

Cursor on a `bar` **use**.

```
TextEdit { range: 0:0 – 0:0,   new_text: "renamed" }   ← insertion at top of file
TextEdit { range: 1:50 – 1:53, new_text: "renamed" }   ← `bar`
TextEdit { range: 1:63 – 1:66, new_text: "renamed" }   ← `bar`
```

The declaration is never touched, and a stray `renamed` is inserted at the very
start of the file:

```kestrel
renamedmodule Test
func foo(bar: lang.i64) -> lang.i64 { let count = renamed; renamed + bar }
```

The file no longer compiles, and the parameter still reads `bar`.

**Cause.** Parameter locals — and `self` — are defined with `Span::synthetic(0)`
(`kestrel-hir-lower/src/lib.rs:71` for `self`, `:79` for each parameter), i.e.
`{ file_id: 0, start: 0, end: 0 }`. That is a zero-width range at the top of
whichever file happens to have id `0`, so the edit lands in a *different file*
than the one being renamed whenever the parameter's file isn't file 0.

## Defect 3 — renaming *from* a declaration renames the enclosing function

Cursor on `count` in `let count = bar;` itself, or on `bar` in the signature —
the most natural rename gesture in both cases.

```
[count decl]     Target::Entity(foo)  → TextEdit { 1:5 – 1:8, "renamed" }
[bar decl (sig)] Target::Entity(foo)  → TextEdit { 1:5 – 1:8, "renamed" }
```

`1:5 – 1:8` is `foo`. The user asked to rename a local; the server renamed the
enclosing function, **workspace-wide**, including every call site in every
other file.

**Cause.** `semantic::hir_expr_at` only sees `HirExpr::Local` *use* sites. A
binding's own identifier is not an expression, and a parameter name is outside
the body entirely, so `hir_expr_at` misses both. `target_at` then fell through
to `semantic::enclosing_decl_at`, which resolves *any* offset inside a
declaration's extent — including its whole body — to that declaration.

### Defect 3b — same fallback, type references in a body (confirmed here)

The audit flagged this as unverified. It reproduces:

```kestrel
func use_it() -> lang.i64 { let x: Foo = Foo(a: 1); x.a }
```

Cursor on `Foo` in the annotation resolved to `Target::Entity(use_it)` — same
`enclosing_decl_at` fallback, same workspace-wide rename of the enclosing
function. `rename::target_at` is the only one of the three `target_at` copies
without a `crate::types::type_at_cursor` pre-check;
`handlers/references.rs` and `handlers/document_highlight.rs` both have one and
so resolve `Foo` correctly.

Guard B turns this into a refusal for free. Making it *rename `Foo`* is a
follow-up: give `rename::target_at` the `type_at_cursor` branch the other two
already have.

## The same fallback in two more handlers

`handlers/references.rs` and `handlers/document_highlight.rs` carry
byte-for-byte copies of the `hir_expr_at → else → enclosing_decl_at` fallback.
Neither writes to disk, so the symptom is "find-all-references on a `let`
binding lists the enclosing function's references" — wrong, not destructive.
Same root cause, so Stage 1 applies the identical guard to all three.

## Fix (Stage 1)

**Guard A — `identifier_for_target` refuses spans that don't spell their name.**
`references::span_spells_name(source, span, name)` is a plain text-equality
check; `identifier_for_target` resolves the target's file via `entity_file` +
`FilePath`, looks the source up in the `sources` map the handler already holds,
and returns `None` when the bytes under the span aren't exactly the name.
Applied to `Target::Local` and, as defence in depth, to `Target::Entity`.

This closes defects 1 and 2, plus `self`, plus every desugaring temp
(`$iter`, `$try_value`, `$dsi`, `$opts`, `$let_tmp`, `_cparam_N`).

**Guard B — the `enclosing_decl_at` fallback requires the decl's own name.**
`references::decl_at_name_offset` wraps `enclosing_decl_at` and keeps the
result only when the offset falls inside `get_name_span(cst, file_id)` — the
same function `identifier_for_target` already uses to find the text it would
edit, so the two agree by construction. No new span data required. Applied in
all three `target_at` copies.

This closes defect 3 and 3b in rename, and the UX bug in find-references and
document-highlight.

## Staged plan

| stage | change | unlocks |
| --- | --- | --- |
| **1** (this) | Guards A + B | renames fail closed instead of corrupting |
| 2 | `Local::name_span` | `let` / `var` / pattern-binding renames work |
| 3 | `AstParam::name_span` | parameter renames work |
| 4 | `semantic::local_decl_at` | renaming *from* a declaration resolves to `Target::Local` |

`self` is never in scope for stages 2-4 — see `decisions.md`.

## Tests

`rename.rs`'s `#[cfg(test)]` module, which had **no** `Target::Local` coverage
at all before this change:

- `local_let_rename_is_refused`
- `parameter_rename_is_refused`
- `self_rename_is_refused`
- `desugared_local_rename_is_refused`
- `rename_from_let_pattern_declaration_is_refused`
- `rename_from_parameter_declaration_is_refused`
- `rename_from_type_reference_in_body_is_refused`

Every one asserts a *refusal*, which is the Stage-1 target and **not** the
intended end state. When stages 2-4 land, these assertions should be flipped to
assert the correct edits — not deleted. Each test's doc comment says so.
