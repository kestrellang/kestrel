# F2 — decisions

## Why text equality, not a synthetic/`$`-prefix heuristic

The obvious guard is "refuse if `span.is_synthetic()` or
`name.starts_with('$')`". It leaks.

Closure-destructure parameters (`kestrel-hir-lower/src/expr.rs:1445`) are
defined as:

```rust
let local = self.define_local(&name, param_is_mut, span.clone());
```

with `name = "_cparam_N"` and `span` = the **whole closure's** span. That local
therefore has a *real, non-synthetic* span and a name that does **not** start
with `$` — both heuristics wave it through, and the span it hands to the
renamer is a whole closure expression. Renaming it would replace the closure
with the new identifier.

`span_spells_name(source, span, name)` — `source.get(span.start..span.end) ==
Some(name)` — tests the property that actually matters: *the bytes we are about
to overwrite are exactly the identifier we claim to be renaming*. It is correct
for every local that exists today and for every local anyone adds later,
regardless of how the span was constructed or what the temp is called. A
heuristic over span provenance or naming convention has to be re-audited every
time either changes; this doesn't.

Placed in `references.rs` next to `entity_file` / `clip_to_identifier`, because
all three are "how do I turn a span into something safe to edit?" helpers
shared by rename, find-references and document-highlight.

## `self` is a permanent refusal, not a Stage-1 stopgap

Every other refusal in Stage 1 is temporary: stages 2-4 give locals and
parameters real name spans and the refusals become working renames. `self` is
different — it can **never** acquire one.

`ReceiverKind` (`lib/kestrel-ast-builder/src/components.rs:205`) is:

```rust
pub enum ReceiverKind { Borrowing, Mutating, Consuming }
```

It is derived purely from the presence of the `mutating` / `consuming`
keywords on the declaration. There is no `self:` token in a Kestrel signature —
the receiver is implied by the method being in a type's scope, not spelled. So
there is no source range to point a name span at, and nothing to rewrite even
if the user asks. `identifier_for_target` returning `None` for `self` is the
permanent, correct answer, and `self_rename_is_refused` should stay an
assertion of refusal forever.

## Guard B reuses `get_name_span` rather than adding span data

`decl_at_name_offset` re-derives the decl's name span with
`kestrel_syntax_tree::utils::get_name_span(&cst.0, decl_span.0.file_id)` —
exactly what `identifier_for_target` independently computes for
`Target::Entity`. Two consequences worth keeping:

1. **No new data.** Stage 1 needed no changes below the LSP crate. That is what
   made it separable from stages 2-4.
2. **The two agree by construction.** "Where the cursor must be for this decl to
   be the target" and "what text rename would edit" are the same span, from the
   same function. If `get_name_span` ever returns `None` for a decl kind, that
   decl becomes unrenameable *and* unclickable together, rather than clickable
   but renaming the wrong bytes.

## The staged plan, and what each stage needs

**Stage 2 — `Local::name_span`.** `kestrel_hir::res::Local` gains a name span
alongside `span`. Feeding it requires token-derived spans on the AST patterns
that produce bindings, because none of them currently narrows to the
identifier:

- `AstPat::Binding` — `lower_binding_pattern`
  (`kestrel-ast-builder/src/lower.rs:1567`) sets `span = self.span(node)`, the
  whole `BindingPattern` node. For `var count` that covers **`"var count"`**,
  not `"count"`. (The audit's diagnosis called this span "exact"; that only
  holds for a non-`mut` binding, where the node happens to contain nothing but
  the identifier. Do not rely on it.) The identifier token is already found
  three lines below — capture its `text_range()` as `name_span` there.
- `AstPat::At`, `AstPat::Array`'s `rest` binding, and `StructPatField` need the
  same treatment; each defines a local through `pat.rs`.

**Stage 3 — `AstParam::name_span`.** Replaces the `Span::synthetic(0)` at
`kestrel-hir-lower/src/lib.rs:79`.

**Stage 4 — `semantic::local_decl_at`.** A declaration-site lookup that maps an
offset to the `LocalId` it binds, so `target_at` can return `Target::Local`
instead of falling through to Guard B's refusal. Needs stages 2/3 first —
there is nothing to match an offset against until name spans exist.

## Four other handlers `Local::name_span` will repair

Recording these so stage 2's value is visible when it's scheduled. All four are
bugs *today*; none is fixed by Stage 1, because Stage 1 only makes rename
refuse.

- **`handlers/definition.rs:110-114`** — go-to-definition on a parameter builds
  `Target::Local { span: local.span }` from the synthetic `0..0` span, so it
  jumps to offset 0 of file id 0, i.e. **the wrong file**. On a `let` local it
  selects the whole statement.
- **`handlers/document_highlight.rs:188`** — pushes `hir.locals[id].span` as
  the declaration highlight, so highlighting a local paints the entire `let`
  statement; for a parameter it paints a zero-width range at file top.
- **`handlers/references.rs:187`** — same span, same result, in the
  `include_declaration` branch of find-all-references.
- **`handlers/code_actions.rs:219`** — the `let` → `var` quickfix takes
  `local.span.start` and searches **backward up to 20 characters** for the
  literal `"let"`:

  ```rust
  let decl_start = local.span.start;
  let search_start = decl_start.saturating_sub(20);
  let prefix = &source[search_start..decl_start];
  let Some(let_in_prefix) = prefix.rfind("let") else { continue };
  ```

  But `local.span.start` **is** the `l` of its own `let` — the span is the
  whole statement. The prefix therefore never contains the keyword it is
  looking for, so the quickfix either silently does nothing or, worse, finds a
  *previous* `let` within those 20 characters and rewrites **that** one.
  This hack should be **deleted** when `name_span` lands, not repaired: with a
  real name span the keyword is found by walking the statement span forward, or
  better, by reading the `Var` token off the CST directly.

## `Local::span` must not be repurposed

Tempting shortcut for stage 2: narrow `Local::span` to the identifier instead
of adding a second field. Don't — two consumers depend on it being **wide**,
and wide is the better answer there:

- `kestrel-type-infer/src/solver.rs:267-274` anchors
  `InferError::CannotInferType` on `local.span`, and also dedupes by
  `(file_id, range)`. Underlining the whole `let count = …;` statement is the
  right diagnostic; underlining just `count` would hide the initializer the
  user needs to see, and narrowing the key would change dedup behaviour.
- `kestrel-mir-lower/src/body/mod.rs:3525-3533` uses `local.span` as the
  fallback anchor for E497 ("ref binding cannot stay live across a
  control-flow merge") when the value has no span of its own.

Stage 2 adds `name_span` as a *new* field. `span` keeps its current meaning.

## Follow-ups filed, not done here

- **`target_at` is triplicated** across `handlers/rename.rs`,
  `handlers/references.rs` and `handlers/document_highlight.rs`. Stage 1
  deliberately copied the guard into all three rather than consolidating,
  because they have real behavioural differences: rename rejects
  `HirExpr::OverloadSet` (ambiguous — which overload?) while the other two
  accept it, and the other two run a `crate::types::type_at_cursor` pre-check
  that rename lacks. Unifying them means reconciling those differences, which
  is a behaviour change, not a refactor — wrong thing to bundle into a
  fail-closed safety fix. Consolidation should happen after stage 4, when the
  three have stopped diverging.
- **`rename::target_at` has no `type_at_cursor` branch.** That is why a cursor
  on `Foo` in `let x: Foo = …` reached the `enclosing_decl_at` fallback at all
  (see `problem.md`, defect 3b). Guard B makes it refuse; adding the branch the
  other two handlers already have would make it rename `Foo` correctly. Natural
  companion to the consolidation above.
