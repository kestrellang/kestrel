---
name: feature
description: End-to-end workflow for adding a new language feature to the lib Kestrel compiler. Use when the user says "add feature X", "implement X in the compiler", or otherwise asks for a non-trivial language change that spans multiple pipeline stages (lexer → parser → AST → name-res → HIR → type-infer → MIR/codegen) and wants it done with design/plan/test gates. For single-stage tweaks or bug fixes, use the `change` skill instead. For writing the `.ks` testdata, use `write-tests`; for writing Kestrel source examples, use `write-kestrel`. For tracing "where does X live in the pipeline?", use `kestrel-pipeline`.
---

# Adding a Feature to the lib Kestrel Compiler

A staged workflow with **confirmation gates** at each phase. The point of the
gates is to avoid writing 800 lines of code against a design the user didn't want.
Do not skip them.

## lib pipeline (where your code will go)

```
Source → Tokens → CST → AST (ECS) → Name Res → HIR → Type Infer → MIR → Codegen
         lexer    parser  ast-     name-res    hir-  type-       mir-  codegen-
                          builder              lower infer       lower cranelift
```

Orthogonal phases:
- **Analyzers** (`kestrel-analyze`) run BodyCheck/DeclCheck passes over AST+HIR
  and emit diagnostics. Validation lives here, not in a distinct "validate" crate.
- **Pattern matching** (`kestrel-pattern-matching`) is called from HIR lowering
  and MIR lowering for `match`-family constructs.

Before you start, skim the target crate's `docs/architecture.md` — lib requires
one per crate (see `lib/AGENTS.md`). `kestrel-pipeline` is faster for "which
file owns variant X?".

## Phase 1 — Brainstorm

Launch an Explore subagent to gather context; do not go it alone. The Explore
agent should return:

- **Similar features** already implemented — what pattern should this feature mirror?
- **Touched crates and files** — which pipeline stages need changes?
- **Interactions** — what existing features does this overlap with (generics,
  extensions, protocols, closures, pattern matching)?
- **Error surfaces** — what new diagnostics does this introduce; what existing
  ones does it break?
- **Test precedents** — how is a similar feature tested under
  `lib/kestrel-test-suite/testdata/`?

Then have a short Socratic back-and-forth with the user: surface ambiguities,
edge cases, and any alternative framings the Explore run turned up. Don't pad
it — one or two rounds, aimed at the parts where "how should this behave?" has
more than one reasonable answer.

> **Gate 1 — design direction.** The user confirms the shape before you write
> a design doc. If they redirect, re-run Explore on the new framing rather than
> bolting changes onto a stale picture.

## Phase 2 — Design document

Write `docs/plans/{feature_name}/{feature_name}-design.md`:

```markdown
# {Feature Name} Design

## Overview
One paragraph: what it does, why it's needed.

## Syntax
```kestrel
// Example
```

## Semantics
- What it means at the language level
- How it interacts with generics / protocols / extensions / pattern matching
- Copy vs move, access modes, visibility implications

## Pipeline impact
| Stage | Change |
|-------|--------|
| Lexer | New tokens? |
| Parser / CST | New SyntaxKind? |
| AST (ECS) | New components / AstExpr variant / AstStmt variant? |
| Name Res | Any new scope or visibility rules? |
| HIR | New HirExpr / HirStmt / HirPat variant? Desugaring target? |
| Type Infer | New constraint kind? Oracle changes? |
| Analyzers | New DeclCheck / BodyCheck? |
| MIR / Codegen | New lowering shape? |

## Error cases
| Condition | Diagnostic (full expected message) |
|-----------|-----------------------------------|

## Edge cases
- List them; say how each is handled.

## Open questions (resolved)
- What came up in brainstorm and how it got resolved.
```

> **Gate 2 — design doc.** User confirms before planning.

## Phase 3 — Implementation plan

Write `docs/plans/{feature_name}/{feature_name}-plan.md`. Order is tests first,
then pipeline stages in the order data flows through them. Skip stages that
don't apply (most features don't touch every stage).

```markdown
# {Feature Name} Implementation Plan

## Test strategy
- Diagnostic tests (one per distinct error, full `// ERROR:` message)
- Execution tests (happy-path behavior, edge cases)
- Location: `lib/kestrel-test-suite/testdata/<category>/<feature>/`
- See `write-tests` skill for the `.ks` testdata format.

## Phases

### Phase 0 — Tests (fail first)
- [ ] Diagnostic `.ks` files under `testdata/diagnostics/<feature>/` or the
      most specific category (e.g. `expressions/<feature>/`)
- [ ] Execution `.ks` files with `// expect-exit: 0`
- [ ] Run via `/triage` with a targeted pattern; expect failures.

### Phase 1 — Lexer (only if adding tokens/keywords)
Files: `lib/kestrel-lexer/src/...`
- [ ] Add token; update keyword table if it's a keyword.

### Phase 2 — CST / SyntaxKind (only if adding a new syntactic shape)
Files: `lib/kestrel-syntax-tree/src/...`
- [ ] Add `SyntaxKind` variant(s).

### Phase 3 — Parser
Files: `lib/kestrel-parser/src/...`
- [ ] Parse rule for the new shape.
- [ ] Wire into the existing declaration / expression / statement dispatcher
      so the parser actually reaches it.
- [ ] **Statement-like expressions.** If the new expression can stand alone as
      a statement without a trailing semicolon (like `if`, `while`, `loop`,
      `for`, `match`), add its variant to `is_statement_like_expr()` in
      `kestrel-parser/src/block/mod.rs`. Forgetting this produces confusing
      "expected semicolon" errors on the preceding expression:
      ```
      for i in range {
          sum = sum + i
      }
      count  // "expected semicolon" on the for loop
      ```
- [ ] **Sub-expressions followed by `{`.** If the new form contains a
      sub-expression that sits directly before a `{` block (like `range` in
      `for i in range { body }`), parse that sub-expression with a
      trailing-closure-less parser such as `condition_binary`. The full
      `expr_parser` will eat the `{` as a trailing-closure argument. See
      `for_expr` in `kestrel-parser/src/expr/mod.rs`.

### Phase 4 — AST (ECS) builder
Files: `lib/kestrel-ast-builder/src/builders/<feature>.rs` (+ `mod.rs`,
`components.rs`, `lower.rs`)
- [ ] AST component(s) if the feature carries new data on an entity.
- [ ] `AstExpr` / `AstStmt` / `AstPat` variant if it's a body-level construct.
- [ ] Builder function; register it in the dispatcher in `lower.rs` /
      `build.rs` / `builders/mod.rs` so the new CST node reaches it.
- [ ] Decls: modules own the entity; files attach as a `FileId` component.

### Phase 5 — Name resolution
Files: `lib/kestrel-name-res/src/...`
- [ ] Scope / visibility handling if the feature introduces a new binder or
      changes lookup rules.
- [ ] Update auto-import / std-import rules only if explicitly part of the design.

### Phase 6 — HIR lowering
Files: `lib/kestrel-hir-lower/src/{expr,stmt,pat,ty,desugar}.rs`
- [ ] `HirExpr` / `HirStmt` / `HirPat` variant — or desugar to existing ones
      (prefer desugar when the semantics overlap with an existing construct).
- [ ] Lowering function; wire into the `lower_ast_*` dispatcher.
- [ ] Body-only: HIR has no decl nodes — decls stay as ECS + AST components.
      See `feedback_no_hir_decls` memory.

### Phase 7 — Type inference
Files: `lib/kestrel-type-infer/src/...`
- [ ] New `Constraint` kind (if existing ones don't cover this) plus solver
      handling in `solver.rs` / `resolve.rs`.
- [ ] Oracle method (`kestrel-semantics` side) if the feature needs a query
      the solver doesn't have today.
- [ ] Ensure Error paths **poison** result TyVars instead of unifying silently
      — see `cascading_infer_errors` and `solver_poison_overreach` memories.

### Phase 8 — Analyzers (validation)
Files: `lib/kestrel-analyze/src/decl/<feature>.rs` or `.../body/<feature>.rs`
- [ ] DeclCheck / BodyCheck analyzer per distinct error class.
- [ ] Register in the analyzer list.
- [ ] Allocate a new `E3xx` diagnostic ID if applicable — coordinate via the
      nearest `AGENTS.md` to avoid collisions.
- [ ] **Semantic validation belongs here, not in HIR lowering.** The HIR
      lowerer (`kestrel-hir-lower`) transforms syntax into semantic trees and
      resolves names/types; it does not decide whether a program is
      well-formed. Type checking, pattern validation, and any other semantic
      correctness checks are analyzer passes. Keeping the split clean makes
      validation logic independently testable and keeps the lowerer focused
      on tree construction.

### Phase 9 — MIR lowering / codegen
Files: `lib/kestrel-mir-lower/src/...`, `lib/kestrel-codegen-cranelift/src/...`
- [ ] MIR shape (`Rvalue` / `Terminator` / witness calls).
- [ ] Cranelift lowering if the feature produces new MIR.
- [ ] Watch for monomorphization edge cases — see
      `monomorphizer_unresolved_typeparam` memory.

## Verification
- [ ] Targeted triage run is green.
- [ ] Full triage run before commit (use `/triage` — never `cargo test` directly).
- [ ] `cargo fmt` + `cargo clippy -p <changed crates>` clean.
- [ ] Relevant crate `docs/architecture.md` updated if pipeline position or
      core types changed.
```

> **Gate 3 — plan.** User confirms before implementation.

## Phase 4 — Implementation

Execute phases in order. After each phase, run **targeted** triage on the tests
relevant to that phase and report the result before moving on. Full triage runs
are for pre-commit, not per-edit — see the project `CLAUDE.md` testing rules.

House rules (pulled from `CLAUDE.md` and memory):
- **Never modify a test to make it pass.** Syntactically invalid tests are the
  only exception, and only with explicit go-ahead from the user.
- **Never `#[ignore]` a test.** If it's broken, fix it or surface it.
- **Don't revert changes when stuck.** After 3 failed attempts at the same
  fix, stop, summarize what was tried and ruled out, and ask for guidance.
- **Debug tracing**: use `debug_trace!` plus `VERBOSE_DEBUG_OUTPUT=1`. Don't
  add `eprintln!` / `println!` for debugging.
- **Multi-agent safety**: other agents may be running. Use `/triage` patterns
  scoped to your feature; don't assume exclusive access.
- **Watch for reusable patterns** during implementation — invariants, ordering
  constraints, new diagnostic IDs, "always update these three places when
  adding a W." Ask the user whether to capture each in the nearest
  `AGENTS.md`. Don't batch them up at the end.

## Phase 5 — Documentation

After tests are green:

1. **User-facing docs.** Write `docs/language/{feature}.md` describing the
   feature for Kestrel users (syntax, semantics, examples, gotchas).
2. **Crate architecture docs.** Update `docs/architecture.md` in any crate
   whose pipeline position, core types, or module map changed. Topic docs
   (`docs/<topic>.md`) get a new entry if the feature warrants a deep dive.
   See `lib/AGENTS.md` for the required structure.
3. **`write-kestrel` skill.** If the feature introduces new syntax or a new
   gotcha future writers should know, update
   `.claude/skills/write-kestrel/SKILL.md`.
4. **Tracking.** Check off items in `ROADMAP.md` / `TODO.md` if they exist for
   this feature.

## When to use a different skill instead

- **`change`** — behavioral tweak to an existing feature (one pipeline stage,
  maybe two). Skip brainstorm and design; go straight to the change.
- **`debug-kestrel` / `debug-test`** — diagnosing a failure, not adding a
  feature.
- **`write-tests`** — writing `.ks` testdata (this skill tells you *what* to
  test; that skill tells you *how* to encode it).
- **`kestrel-pipeline`** — "where does X live?" lookups.
