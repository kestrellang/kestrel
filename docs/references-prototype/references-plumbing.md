# References — Front-End Plumbing Checklist

Every item was read from the tree at 2026-06-03. Line numbers for churning files (expand.rs, mir-lower/body/\*) should be re-verified before use.

---

## Stage 0 — Lexer / syntax tree (rowan u16 IDs)

### Lexer: `lib/kestrel-lexer/src/lib.rs`

The `Token` enum uses `logos`. `&` is already lexed as `Token::Ampersand` (line 728). `mutating` is `Token::Mutating` (line 491). No new tokens are needed for the MVP `&T` / `&mutating T` syntax; both tokens already exist.

**No changes required in the lexer.**

### SyntaxKind: `lib/kestrel-syntax-tree/src/lib.rs`

The enum starts at line 37. The rowan u16 stability rule: **append new variants at the end of the enum, never insert in the middle**. All existing discriminants are positionally ordered (rowan stores them as raw u16; any insertion renumbers successors and corrupts stored green trees).

Current last type-node variant is `TySome` (line 119). The next block starts with `Path` (line 122).

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-syntax-tree/src/lib.rs:119` — append `TyRef` and `TyMutRef` **after** `TySome` and before `Path`. Example:
  ```
  TySome,     // some P - opaque type
  TyRef,      // &T - shared reference
  TyMutRef,   // &mutating T - mutable reference
  ```

The `From<Token> for SyntaxKind` match (line 346) does not need a branch for `TyRef`/`TyMutRef` because they are composite parser-emitted nodes, not single-token terminals. But the `kind_from_raw` reverse-mapping table starting at line 467 must be updated:

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-syntax-tree/src/lib.rs:467` (the `kind_from_raw` dispatch block) — add `const TY_REF: u16 = SyntaxKind::TyRef as u16;` and `const TY_MUT_REF: u16 = SyntaxKind::TyMutRef as u16;` entries so the rowan round-trip recognises the new node kinds.

---

## Stage 1 — Parser: `lib/kestrel-parser/src/ty/mod.rs`

### `TyVariant` enum (line 473)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-parser/src/ty/mod.rs:473` — add two variants:
  ```rust
  /// &T — shared borrow reference
  Ref(Span, Box<TyVariant>),         // (ampersand_span, inner)
  /// &mutating T — mutable reference
  MutRef(Span, Span, Box<TyVariant>), // (ampersand_span, mutating_span, inner)
  ```

### Parser combinators — `ty_parser` function (line 160)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-parser/src/ty/mod.rs:338` — add a `ref_type` parser alternative **before** `base_ty` assembly (line 339). Pattern: consume `Token::Ampersand`, optionally consume `Token::Mutating`, then recurse into `ty`. Insert into the `base_ty` chain:
  ```rust
  let ref_type = skip_trivia()
      .ignore_then(just(Token::Ampersand).map_with(|_, e| to_kestrel_span(e.span())))
      .then(
          skip_trivia()
              .ignore_then(just(Token::Mutating).map_with(|_, e| to_kestrel_span(e.span())).or_not()),
      )
      .then(ty.clone())
      .map(|((amp, mutating), inner)| match mutating {
          Some(mut_span) => TyVariant::MutRef(amp, mut_span, Box::new(inner)),
          None => TyVariant::Ref(amp, Box::new(inner)),
      });
  ```
  Add `ref_type.or(...)` as the first alternative so it takes priority over `path`.

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-parser/src/ty/mod.rs:419` — add arms to `emit_ty_variant`:
  ```rust
  TyVariant::Ref(amp_span, inner) => emit_ref_type(sink, amp_span.clone(), inner, false),
  TyVariant::MutRef(amp_span, mut_span, inner) => emit_ref_type_mut(sink, amp_span.clone(), mut_span.clone(), inner),
  ```

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-parser/src/ty/mod.rs` — add `emit_ref_type` / `emit_ref_type_mut` helpers that open `SyntaxKind::Ty` / `SyntaxKind::TyRef` (or `TyMutRef`), emit the `Ampersand` token, optionally the `Mutating` token, then recurse via `emit_ty_variant` for the inner type, then close both nodes.

### `TyExpression` helpers (line 19)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-parser/src/ty/mod.rs:19` — add `is_ref()` and `is_mut_ref()` query methods mirroring the existing `is_never()`, `is_tuple()` etc., checking for `SyntaxKind::TyRef` / `TyMutRef`.

---

## Stage 2 — AST type: `lib/kestrel-ast/src/ast_type.rs`

### `AstType` enum (line 43)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-ast/src/ast_type.rs:43` — add:
  ```rust
  /// Shared reference: &T
  Ref {
      inner: Box<AstType>,
      span: Span,
  },
  /// Mutable reference: &mutating T
  MutRef {
      inner: Box<AstType>,
      span: Span,
  },
  ```

### `ast_type_span` helper in `lib/kestrel-hir-lower/src/ty.rs` (line 716)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-hir-lower/src/ty.rs:716` — add arms to the exhaustive match inside `ast_type_span`:
  ```rust
  AstType::Ref { span, .. } | AstType::MutRef { span, .. } => span.clone(),
  ```

### AST builder: CST → `AstType` extraction

The AST builder crate (not enumerated above) reads SyntaxKind nodes to produce `AstType`. Find the function that dispatches on `SyntaxKind::TyPath`, `SyntaxKind::TyTuple`, etc. and add:

- [ ] `lib/kestrel-ast/src/` (the builder/extraction function matching `SyntaxKind`) — add arms for `SyntaxKind::TyRef` → `AstType::Ref { .. }` and `SyntaxKind::TyMutRef` → `AstType::MutRef { .. }`.

---

## Stage 3 — HIR type: `lib/kestrel-hir/src/ty.rs`

### `HirTy` enum (line 17)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-hir/src/ty.rs:17` — add:
  ```rust
  /// Shared reference: &T. Carries the referent type and span.
  Ref {
      inner: Box<HirTy>,
      span: Span,
  },
  /// Mutable reference: &mutating T.
  MutRef {
      inner: Box<HirTy>,
      span: Span,
  },
  ```

### `override_span` in `lib/kestrel-hir-lower/src/ty.rs` (line 389)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-hir-lower/src/ty.rs:389` — add arms to the exhaustive `match ty` inside `override_span`:
  ```rust
  HirTy::Ref { span: s, .. } | HirTy::MutRef { span: s, .. } => *s = span.clone(),
  ```

### `contains_opaque` in `lib/kestrel-hir-lower/src/ty.rs` (line 409)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-hir-lower/src/ty.rs:409` — add fall-through arms (refs don't contain opaque types; use the `_ => false` default, but the match must remain exhaustive — add explicit arms):
  ```rust
  HirTy::Ref { inner, .. } | HirTy::MutRef { inner, .. } => contains_opaque(inner),
  ```

---

## Stage 4 — HIR lowering: `lib/kestrel-hir-lower/src/ty.rs`

### `lower_ast_type` (line 28)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-hir-lower/src/ty.rs:28` — add arms to the exhaustive `match ty`:
  ```rust
  AstType::Ref { inner, span } => HirTy::Ref {
      inner: Box::new(lower_ast_type(ctx, owner, root, inner)),
      span: span.clone(),
  },
  AstType::MutRef { inner, span } => HirTy::MutRef {
      inner: Box::new(lower_ast_type(ctx, owner, root, inner)),
      span: span.clone(),
  },
  ```

### `lower_type_replacing_opaque` (line 340)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-hir-lower/src/ty.rs:340` — add arms:
  ```rust
  HirTy::Ref { inner, span } if contains_opaque(inner) => HirTy::Ref {
      inner: Box::new(lower_type_replacing_opaque(ctx, inner, concrete)),
      span: span.clone(),
  },
  HirTy::MutRef { inner, span } if contains_opaque(inner) => HirTy::MutRef {
      inner: Box::new(lower_type_replacing_opaque(ctx, inner, concrete)),
      span: span.clone(),
  },
  ```

---

## Stage 5 — Type inference: `lib/kestrel-type-infer/src/ty.rs`

### `TyKind` enum (line 31)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/ty.rs:31` — add:
  ```rust
  /// Shared reference: &T.
  Ref { inner: TyVar },
  /// Mutable reference: &mutating T.
  MutRef { inner: TyVar },
  ```

### `TyKind::entity()` method (line 98)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/ty.rs:98` — the existing `_ => None` wildcard covers the new variants. Verify the wildcard is still present after adding variants; no explicit arm needed.

### `TyKind::args()` method (line 111)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/ty.rs:111` — the existing `_ => &[]` wildcard covers the new variants. Verify; no explicit arm needed.

### `TyKind::is_nominal_concrete()` method (line 122)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/ty.rs:122` — uses `matches!`; Ref/MutRef are not nominal, so this is unchanged. Verify no exhaustive issue.

---

## Stage 6 — Type inference: `lib/kestrel-type-infer/src/unify.rs`

### `unify_concrete` (line 158)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/unify.rs:158` — add structural unification for references:
  ```rust
  (TyKind::Ref { inner: a }, TyKind::Ref { inner: b }) => unify(ctx, *a, *b),
  (TyKind::MutRef { inner: a }, TyKind::MutRef { inner: b }) => unify(ctx, *a, *b),
  // Ref ≠ MutRef — shared and mutable references don't unify with each other.
  ```
  These must appear before the final wildcard `_ => Err(UnifyError::Mismatch)`.

---

## Stage 7 — Type inference: `lib/kestrel-type-infer/src/compare.rs`

### `normalize_hir_type` (line 88)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/compare.rs:88` — add arms to the exhaustive `match ty`:
  ```rust
  HirTy::Ref { inner, .. } => ResolvedTy::Ref {
      inner: Box::new(normalize_hir_type(qctx, root, inner, env, state)),
  },
  HirTy::MutRef { inner, .. } => ResolvedTy::MutRef {
      inner: Box::new(normalize_hir_type(qctx, root, inner, env, state)),
  },
  ```
  This requires `ResolvedTy::Ref` / `ResolvedTy::MutRef` to exist first (Stage 8).

---

## Stage 8 — Type inference result: `lib/kestrel-type-infer/src/result.rs`

### `ResolvedTy` enum (line ~90)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/result.rs:90` — add:
  ```rust
  /// Shared reference: &T.
  Ref { inner: Box<ResolvedTy> },
  /// Mutable reference: &mutating T.
  MutRef { inner: Box<ResolvedTy> },
  ```

### `kind_to_resolved` (line 155)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/result.rs:155` — add arms:
  ```rust
  TyKind::Ref { inner } => ResolvedTy::Ref {
      inner: Box::new(resolve_to_concrete(ctx, *inner)),
  },
  TyKind::MutRef { inner } => ResolvedTy::MutRef {
      inner: Box::new(resolve_to_concrete(ctx, *inner)),
  },
  ```

### `contains_error` (line 236)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/compare.rs:236` — add arms:
  ```rust
  ResolvedTy::Ref { inner } | ResolvedTy::MutRef { inner } => contains_error(inner),
  ```

### `substitute_resolved_ty` in `lib/kestrel-mir-lower/src/ty.rs` (line 427)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir-lower/src/ty.rs:427` — add arms:
  ```rust
  ResolvedTy::Ref { inner } => ResolvedTy::Ref {
      inner: Box::new(substitute_resolved_ty(inner, type_params, args)),
  },
  ResolvedTy::MutRef { inner } => ResolvedTy::MutRef {
      inner: Box::new(substitute_resolved_ty(inner, type_params, args)),
  },
  ```

---

## Stage 9 — Type inference: constraint generation and solver

The solver generates constraints by matching `TyKind` / `HirTy`. References need type-checking but no new constraint *kind* in Stage 1 (no outlives). The sites to check are in `lib/kestrel-type-infer/src/generate.rs` and `solver.rs`, wherever `HirTy` or `TyKind` is matched exhaustively:

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/generate.rs` — audit every `match ty` / `match hir_ty` for exhaustiveness on `HirTy`. Add `HirTy::Ref` / `HirTy::MutRef` arms. In constraint generation, a reference expression `&x` produces a `TyKind::Ref { inner }` where `inner` is the type of `x`. No coercion constraint is needed between `Ref` and `MutRef` in Stage 1.

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-type-infer/src/solver.rs` — audit every `match kind` for exhaustiveness. Add `TyKind::Ref` / `TyKind::MutRef` arms (typically fall through to no-op or existing handling).

---

## Stage 10 — MIR type: `lib/kestrel-mir/src/ty.rs`

### `MirTy` enum (line 15)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/ty.rs:15` — add:
  ```rust
  /// Shared reference: &T. Machine-isomorphic to Pointer(T) but distinct
  /// so drop/clone elaboration can identify it. No drop shim; clone = bitwise copy.
  Ref(TyId),
  /// Mutable reference: &mutating T. Same machine shape as Ref.
  MutRef(TyId),
  ```
  **Do not reuse `MirTy::Pointer`.** See the feasibility doc §7 trap #3.

### `TyArena` convenience constructors (line 60)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/ty.rs:127` (after `pointer`) — add:
  ```rust
  pub fn ref_ty(&mut self, inner: TyId) -> TyId { self.intern(MirTy::Ref(inner)) }
  pub fn mut_ref_ty(&mut self, inner: TyId) -> TyId { self.intern(MirTy::MutRef(inner)) }
  ```

---

## Stage 11 — MIR ty_query: `lib/kestrel-mir/src/ty_query.rs`

### `copy_behavior` (line 6)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/ty_query.rs:14` — add `MirTy::Ref(_) | MirTy::MutRef(_)` to the bitwise-copy arm (references have pointer-width scalar layout; they are not dropped and are bitwise-copyable):
  ```rust
  | MirTy::Ref(_)
  | MirTy::MutRef(_)
  ```

### `copy_is_mono_dependent` (line 173)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/ty_query.rs:179` — the existing `_ => false` wildcard covers `Ref`/`MutRef`. Verify it remains; references are never mono-dependent.

### `needs_drop` (line 235)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/ty_query.rs:237` — add `MirTy::Ref(_) | MirTy::MutRef(_)` to the `false` arm (references are borrowed, not owned; never dropped):
  ```rust
  | MirTy::Ref(_)
  | MirTy::MutRef(_)
  ```

---

## Stage 12 — MIR substitute: `lib/kestrel-mir/src/substitute.rs`

### `substitute` (line 24)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/substitute.rs:24` — add arms (after `MirTy::Pointer`):
  ```rust
  MirTy::Ref(inner) => {
      let sub = substitute(arena, inner, subst);
      if sub != inner { arena.ref_ty(sub) } else { ty }
  },
  MirTy::MutRef(inner) => {
      let sub = substitute(arena, inner, subst);
      if sub != inner { arena.mut_ref_ty(sub) } else { ty }
  },
  ```

---

## Stage 13 — MIR display: `lib/kestrel-mir/src/display.rs`

### `fmt_ty` (line 183)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/display.rs:183` — add arms:
  ```rust
  MirTy::Ref(inner) => format!("&{}", fmt_ty(*inner, arena, module)),
  MirTy::MutRef(inner) => format!("&mutating {}", fmt_ty(*inner, arena, module)),
  ```

---

## Stage 14 — MIR-lower type lowering: `lib/kestrel-mir-lower/src/ty.rs`

### `lower_type` (line 111)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir-lower/src/ty.rs:111` — add arms:
  ```rust
  HirTy::Ref { inner, .. } => {
      let inner_ty = lower_type(ctx, inner);
      ctx.intern(MirTy::Ref(inner_ty))
  },
  HirTy::MutRef { inner, .. } => {
      let inner_ty = lower_type(ctx, inner);
      ctx.intern(MirTy::MutRef(inner_ty))
  },
  ```

### `lower_resolved_ty` (line 168)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir-lower/src/ty.rs:168` — add arms:
  ```rust
  ResolvedTy::Ref { inner } => {
      let inner_ty = lower_resolved_ty(ctx, inner);
      ctx.intern(MirTy::Ref(inner_ty))
  },
  ResolvedTy::MutRef { inner } => {
      let inner_ty = lower_resolved_ty(ctx, inner);
      ctx.intern(MirTy::MutRef(inner_ty))
  },
  ```

### `lower_type_replacing_opaque` (line 340)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir-lower/src/ty.rs:340` — the `_ => lower_type(ctx, ty)` fallthrough already handles `Ref`/`MutRef` that don't contain opaque types. Add explicit arms only if `contains_opaque(inner)` (same pattern as `AssocProjection` arm at line 393):
  ```rust
  HirTy::Ref { inner, span } if contains_opaque(inner) => {
      let inner_ty = lower_type_replacing_opaque(ctx, inner, concrete);
      ctx.intern(MirTy::Ref(inner_ty))
  },
  HirTy::MutRef { inner, span } if contains_opaque(inner) => {
      let inner_ty = lower_type_replacing_opaque(ctx, inner, concrete);
      ctx.intern(MirTy::MutRef(inner_ty))
  },
  ```

---

## Stage 15 — MIR mono expand: `lib/kestrel-mir/src/mono/expand.rs`

The `expand_destroy_copy` pass pattern-matches `MirTy` to decide whether to insert drop-shim calls or clone calls.

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/mono/expand.rs` — find every exhaustive `match arena.get(ty)` inside `expand_function` (and helpers). Add `MirTy::Ref(_) | MirTy::MutRef(_)` to the trivial/no-op arms in both `DestroyValue` expansion and `CopyValue` expansion. References are never dropped and are bitwise-copied (same as `Pointer`).

---

## Stage 16 — MIR passes: drop_shim, copy_check, clone_shim

### `lib/kestrel-mir/src/passes/drop_shim.rs`

- [ ] Audit every `match arena.get(ty)` (the shim-synthesis pass iterates field types). Add `MirTy::Ref(_) | MirTy::MutRef(_)` to the no-drop arm (references are never field owners that need shim-dropping).

### `lib/kestrel-mir/src/passes/copy_check.rs`

- [ ] Audit the `copy_behavior` call chain. `MirTy::Ref`/`MutRef` return `CopyBehavior::Bitwise` (already wired in Stage 11). No change required unless a direct `match arena.get(ty)` exists in this file that needs a new arm.

### `lib/kestrel-mir/src/passes/clone_shim.rs`

- [ ] Audit `match arena.get(field.ty)` in clone-shim field-copy logic. Add `MirTy::Ref(_) | MirTy::MutRef(_)` to the bitwise-copy arm (same as Pointer).

---

## Stage 17 — Codegen: `lib/kestrel-codegen-cranelift/src/ty.rs`

### `TypeCache::classify` (line 78)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-codegen-cranelift/src/ty.rs:78` — add `MirTy::Ref(_) | MirTy::MutRef(_)` alongside `MirTy::Pointer(_)` at line 91:
  ```rust
  MirTy::Pointer(_) | MirTy::Ref(_) | MirTy::MutRef(_) | MirTy::FuncThin { .. } => TypeRepr::Scalar(ptr_ty),
  ```

---

## Stage 18 — Codegen ABI: `lib/kestrel-codegen-cranelift/src/abi.rs`

This is **the core of the `ret_borrow` change** described in the feasibility doc.

### `return_mode` (line 33)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-codegen-cranelift/src/abi.rs:33` — add a `ret_borrow: bool` parameter. When `ret_borrow` is true and repr is `Scalar`, emit `ReturnMode::Direct(ptr_ty)` (raw pointer, not the scalar pointee value). This requires threading `ret_borrow` through from `MonoFunction`:
  ```rust
  pub fn return_mode(repr: TypeRepr, is_main: bool, ret_borrow: bool, ptr_ty: ir::Type) -> ReturnMode {
      if is_main { return ReturnMode::Direct(ir::types::I64); }
      if ret_borrow { return ReturnMode::Direct(ptr_ty); }
      match repr { ... }
  }
  ```

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-codegen-cranelift/src/abi.rs:44` (`build_signature`) — pass the new `ret_borrow` field from `MonoFunction` to `return_mode`.

### `MonoFunction` struct: `lib/kestrel-mir/src/mono/types.rs` (line 77)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/mono/types.rs:77` — add `pub ret_borrow: bool` field (default `false`). This is the per-function return-convention bit described in §6 of the feasibility doc.

---

## Stage 19 — Codegen function compilation: `lib/kestrel-codegen-cranelift/src/func.rs`

### `resolve_scalar` (line 45)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-codegen-cranelift/src/func.rs:45` — when `ret_borrow` is set and the value's type is `MirTy::Ref` or `MirTy::MutRef`, return `get_value(builder, id)` directly (the pointer itself), **not** the loaded scalar. This is the trap described in §5 ("#1 — Conditional `@guaranteed` return"): `resolve_scalar` currently loads through the ByRef pointer for `Guaranteed`+`Scalar`; a scalar `&T` return must use the raw pointer.

### `compile_return` site in `lib/kestrel-codegen-cranelift/src/terminator.rs`

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-codegen-cranelift/src/terminator.rs` — find the `TerminatorKind::Return` arm. When `func.ret_borrow`, use `get_value` (raw pointer) instead of `resolve_scalar`. Mirrors the `mod.rs:477` guard in the MIR lowerer.

---

## Stage 20 — MIR-lower return guards: `lib/kestrel-mir-lower/src/body/mod.rs` and `expr.rs`

These are the two copy-at-return sites that must become conditional on `ret_borrow`.

### Copy guard #1: `lib/kestrel-mir-lower/src/body/mod.rs` (line 478)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir-lower/src/body/mod.rs:478` — wrap the `if self.body.value(value).ownership == Owned { ... emit_copy_value ... }` block in `if !self.current_func_ret_borrow { ... }`. Add `current_func_ret_borrow: bool` to the body-lowering context, set from the function signature when the return type is `HirTy::Ref`.

### Copy guard #2: `lib/kestrel-mir-lower/src/body/expr.rs` (line 258)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir-lower/src/body/expr.rs:258` — same conditional gate: `if !self.current_func_ret_borrow { ... }` around the `Ownership::Guaranteed → emit_copy_value → emit_end_borrow` block in the `HirExpr::Return` arm.

---

## Stage 21 — MIR OSSA verifier: `lib/kestrel-mir/src/verify.rs`

### `try_consume` (line 321)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/verify.rs:321` — currently early-returns `true` for `ownership != Owned`. This correctly no-ops for `@guaranteed` returns today. No change required for Stage 1 (refs returned as `@guaranteed` are not in `self.owned`). But note: once `ret_borrow` is set, the `Return` site at line 988 must additionally assert the returned `@guaranteed` value's `borrow_source` traces to a `Param`. Add a new per-function `ret_borrow` input to the verifier (passed alongside `func_name` / `entity`) so the return check can be gated.

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/verify.rs:988` — in the `TerminatorKind::Return(v)` arm: when `ret_borrow`, assert `body.value(*v).ownership == Guaranteed` and `borrow_source` transitively traces to a `Param`. This is the escape-checker stub (§5 #2 of feasibility doc).

---

## Stage 22 — MIR-lower: `set_terminator` borrow carve-out

### `set_terminator` in `lib/kestrel-mir-lower/src/body/mod.rs` (~line 1820)

- [ ] `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir-lower/src/body/mod.rs:~1820` — `set_terminator` force-ends **all** scope-tracked borrows before every terminator. When the terminator is `TerminatorKind::Return(v)` and `self.current_func_ret_borrow`, the returning borrow value `v` must be carved out of the force-EndBorrow loop. Exactly one value, no more.

---

## Summary table of files requiring changes

| File | Stage(s) | Nature |
|---|---|---|
| `lib/kestrel-syntax-tree/src/lib.rs` | 0 | New SyntaxKind variants (append only) |
| `lib/kestrel-lexer/src/lib.rs` | 0 | No change |
| `lib/kestrel-parser/src/ty/mod.rs` | 1 | New TyVariant, parser combinator, emit helpers |
| `lib/kestrel-ast/src/ast_type.rs` | 2 | New AstType variants |
| `lib/kestrel-hir-lower/src/ty.rs` | 2, 4 | `ast_type_span`, `lower_ast_type`, `lower_type_replacing_opaque`, `substitute_resolved_ty`, `contains_opaque` |
| `lib/kestrel-hir/src/ty.rs` | 3 | New HirTy variants |
| `lib/kestrel-type-infer/src/ty.rs` | 5 | New TyKind variants |
| `lib/kestrel-type-infer/src/unify.rs` | 6 | `unify_concrete` structural arms |
| `lib/kestrel-type-infer/src/compare.rs` | 7 | `normalize_hir_type`, `contains_error` |
| `lib/kestrel-type-infer/src/result.rs` | 8 | New ResolvedTy variants, `kind_to_resolved` |
| `lib/kestrel-type-infer/src/generate.rs` | 9 | All exhaustive HirTy/TyKind matches |
| `lib/kestrel-type-infer/src/solver.rs` | 9 | All exhaustive TyKind matches |
| `lib/kestrel-mir/src/ty.rs` | 10 | New MirTy variants, TyArena helpers |
| `lib/kestrel-mir/src/ty_query.rs` | 11 | `copy_behavior`, `needs_drop` arms |
| `lib/kestrel-mir/src/substitute.rs` | 12 | `substitute` arms |
| `lib/kestrel-mir/src/display.rs` | 13 | `fmt_ty` arms |
| `lib/kestrel-mir-lower/src/ty.rs` | 14 | `lower_type`, `lower_resolved_ty` |
| `lib/kestrel-mir/src/mono/expand.rs` | 15 | Expand arms for Ref/MutRef (no-op) |
| `lib/kestrel-mir/src/passes/drop_shim.rs` | 16 | Field-type match arms |
| `lib/kestrel-mir/src/passes/clone_shim.rs` | 16 | Field-type match arms |
| `lib/kestrel-mir/src/mono/types.rs` | 18 | `MonoFunction.ret_borrow: bool` field |
| `lib/kestrel-codegen-cranelift/src/ty.rs` | 17 | `classify` arm |
| `lib/kestrel-codegen-cranelift/src/abi.rs` | 18 | `return_mode` signature + branch |
| `lib/kestrel-codegen-cranelift/src/func.rs` | 19 | `resolve_scalar` + return path |
| `lib/kestrel-codegen-cranelift/src/terminator.rs` | 19 | `Return` arm |
| `lib/kestrel-mir-lower/src/body/mod.rs` | 20, 22 | Two copy guards + `set_terminator` carve-out |
| `lib/kestrel-mir-lower/src/body/expr.rs` | 20 | Return copy guard |
| `lib/kestrel-mir/src/verify.rs` | 21 | Return-borrow escape check |

---

## Incremental compilation impact

**New `TyKind` variants.** `TyKind` is inside `TySlot::Resolved` stored in `InferCtx::types` (a `Vec`). The `InferCtx` is ephemeral per body invocation and not cached by the hECS query system directly — the queries that cache are `InferBody` (output: `TypedBody`), `LowerTypeAnnotation`, `LowerCallableReturnType`, etc. Adding `TyKind::Ref`/`MutRef` changes the discriminant space of `TyKind` but not the serialized form of `TypedBody` (it contains `ResolvedTy`, not `TyKind`). Adding `ResolvedTy::Ref`/`MutRef` changes the hash of `TypedBody`. All bodies that touch a reference type will be re-inferred on cache-miss. Non-reference bodies are unaffected.

**New `SyntaxKind` variants.** The `SyntaxKind` u16 value is embedded in the rowan green-tree nodes which are the CST cached per file. Appending `TyRef` and `TyMutRef` **after** all existing variants preserves every existing discriminant: no existing cached green tree refers to the new values, so no re-parse of existing files is forced. The rowan `kind_from_raw` table must be updated to round-trip the new values, but this has no effect on existing trees. **Never insert in the middle** — that renumbers successors.

**New `MirTy` variants.** `MirTy` is interned in `TyArena` which is stored in `MirModule` (output of the pipeline pass, not individually cached by entity). The entire module is rebuilt on any relevant change. Adding variants changes the `HashMap<MirTy, TyId>` intern map; the only risk is an equality regression if `MirTy::Ref(x)` collides with `MirTy::Pointer(x)`, which cannot happen because they are distinct enum variants with different discriminants. The `PartialEq + Hash` derives on `MirTy` handle this correctly.

**`MonoFunction.ret_borrow`** is a new `bool` field on a `Debug + Clone` struct that is not hashed (mono functions are identity-keyed by `InstantiationKey`). Adding it does not invalidate existing cached keys; it defaults to `false` for all non-reference-returning functions.

---

## Suggested landing order

Land these stages in sequence so the build stays green at each step:

1. **Stages 0–1 (lexer, SyntaxKind, parser)** — append SyntaxKind variants and add the `TyVariant::Ref`/`MutRef` parser. The CST is built but the AST builder will produce an error for unknown nodes; gate with `todo!()` initially. Build compiles; no tests break.

2. **Stages 2–4 (AST → HIR lowering)** — add `AstType::Ref`/`MutRef` and the `lower_ast_type` arms. Lower to `HirTy::Ref`/`MutRef`. All exhaustive matches in `hir-lower` now compile.

3. **Stages 5–9 (type inference)** — add `TyKind::Ref`/`MutRef`, `ResolvedTy::Ref`/`MutRef`, wire `unify_concrete`, `normalize_hir_type`, `kind_to_resolved`. Gate constraint generation on a feature flag initially. The solver compiles; inference of existing code is unaffected.

4. **Stages 10–16 (MIR)** — add `MirTy::Ref`/`MutRef`, wire all the passes (ty_query, substitute, display, expand, drop_shim, clone_shim). All of these are mechanical no-op arms.

5. **Stage 18 — `ret_borrow` field on `MonoFunction`** — add the field (default false). No behavior change; all call sites compile by supplying `false`.

6. **Stages 19–22 (codegen + return convention)** — wire `return_mode` branch, `resolve_scalar` guard, the two copy guards, and the verifier escape check. This is the dangerous surgery described as #1 in the feasibility doc; land with tests for a simple `func foo(x: Int64) -> &Int64 { return &x }` that the verifier must reject (dangling local), and a `func bar(x: &Int64) -> &Int64 { return x }` that must compile and pass.

**Gate the copy-guard removal (Stage 20) strictly behind the `ret_borrow` bit.** This is the only non-additive change; all other stages are purely additive.
