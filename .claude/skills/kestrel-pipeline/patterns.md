# Patterns — CST → AST → HIR → infer → MIR

Covers every variant of `AstPat` (12) and `HirPat` (12). Verify before citing —
pipeline maps go stale.

Top-level dispatch anchors:

- AST constructor switch: `lib2/kestrel-ast-builder/src/lower.rs:1321` (`lower_pat`)
- HIR lowering switch: `lib2/kestrel-hir-lower/src/pat.rs:34` (`lower_pat_inner`)
- Inference gen switch: `lib2/kestrel-type-infer/src/generate.rs:607` (`gen_pat`, takes
  a `scrutinee_tv: TyVar` and a `source: MatchSource`)
- MIR lowering: patterns are consumed inside `lower_match` at
  `lib2/kestrel-mir-lower/src/body_lower.rs:3319`; there is no stand-alone
  pattern-lowering dispatch.

Patterns never appear in MIR directly — they're compiled into a decision tree by the
`kestrel-pattern-matching` crate, then lowered as `Switch` terminators + binding
assignments.

---

## AstPat variants (12)

Enum: `lib2/kestrel-ast/src/ast_body.rs:226`.

### AstPat::Wildcard

- Surface: `_`.
- CST: `WildcardPattern` (`lower.rs:1326`).
- AST-builder: `lower.rs:1328` — direct alloc of `AstPat::Wildcard { span }`.
- HIR lowering: `pat.rs:35` → `HirPat::Wildcard { span }` (1:1).
- Type-infer: `generate.rs:615-617` — no constraint.
- MIR: consumed by decision tree as "matches anything"; emits no binding.
- Gotchas: `let _ = expr;` routes through the complex-pattern path in
  `lower_let_stmt` (not simple binding) because `AstPat::Wildcard` is not
  `AstPat::Binding`.

### AstPat::Binding

- Surface: `x`, `var x`, `mut x` (binding with mut flag).
- CST: `BindingPattern` (`lower.rs:1330`).
- AST-builder: `lower.rs:1354` (`lower_binding_pattern`). Reads `Var` keyword for
  `is_mut`.
- HIR lowering: `pat.rs:37-43` — `define_local(name, is_mut || force_mut, span)` then
  `HirPat::Binding { local, span }`. `force_mut` propagates outer `var (a, b) = ...`
  mutability into sub-bindings.
- Type-infer: `generate.rs:619-622` — `ctx.local_types.insert(local, scrutinee_tv)`.
- MIR: decision tree emits an `Assign` to `Place::local(binding)` with the matched
  value.
- Gotchas: `var x` binding sets `is_mut` on the local; `x` alone does not — parameter
  label rules don't apply here, these are binding names.

### AstPat::Tuple

- Surface: `(a, b)`, `(a, .., b)`, `(.., b)`, `(a, ..)`.
- CST: `TuplePattern` (`lower.rs:1331`).
- AST-builder: `lower.rs:1371` (`lower_tuple_pattern`). Splits elements around the
  first rest pattern into `prefix` / `suffix`; tracks `has_rest` and `multiple_rests`.
  Single-element tuple-pattern `(pat)` is **grouping**, not a 1-tuple — returns the
  inner pat at `lower.rs:1435-1436`.
- HIR lowering: `pat.rs:45-78`. Validates `multiple_rests` — emits diagnostic if more
  than one `..` was found. Recursively lowers prefix / suffix. Emits
  `HirPat::Tuple { prefix, has_rest, suffix, span }` (the `multiple_rests` flag is
  consumed at HIR lowering — does not survive into HIR).
- Type-infer: `generate.rs:629-668`:
  - `has_rest` → emit `Constraint::TupleRestPat { scrutinee, prefix_tys, suffix_tys }`.
    Deferred until scrutinee is a concrete tuple.
  - no rest → build a fresh tuple TyVar of the pattern's arity and `ctx.equal` against
    the scrutinee. Arity-mismatch is suppressed when the scrutinee already resolved to
    a different-arity tuple (so analyzers E111 / E314 report instead).
  - Recurses into each sub-pattern.
- Solver: `solve_tuple_rest_pat` (2401) for `has_rest`; `solve_equal` (817) for the
  fixed-arity equate.
- MIR: decision tree.
- Gotchas: see `cascading_infer_errors.md` for why arity-mismatch is suppressed here
  (ImplicitPat + TupleRestPat arg poisoning fixes).

### AstPat::Literal

- Surface: `5`, `"text"`, `true`, `'c'`.
- CST: `LiteralPattern` (`lower.rs:1332`).
- AST-builder: `lower.rs:1449` (`lower_literal_pattern`). Stores `LitPatKind`.
- HIR lowering: `pat.rs:80-86` — `lower_lit_pat` (`pat.rs:479`) converts
  `LitPatKind` → `HirLiteral` (integer / float / string / bool / char). Emits
  `HirPat::Literal { value, span }`.
- Type-infer: `generate.rs:624-627` — `literal_to_tyvar(value)` + `ctx.equal(lit_tv,
  scrutinee_tv)`.
- Solver: `solve_equal`.
- MIR: decision-tree equality check; see `match_int64_aggregate_ptr_bug.md` for a
  historical bug where Int64 / UInt64 literal patterns compared the scrutinee pointer
  instead of the value.

### AstPat::Range

- Surface: `1..5`, `1..=10`, `'a'..='z'`, `..5`, `0..`.
- CST: `RangePattern` (`lower.rs:1333`).
- AST-builder: `lower.rs:1473` (`lower_range_pattern`). Detects `inclusive` by
  `DotDotEquals` token.
- HIR lowering: `pat.rs:88-136`. Lowers bounds to `HirLiteral`s and validates:
  integer / char ranges must be `start <= end` (or `<` for exclusive) — otherwise
  emit an "invalid range bounds" diagnostic. Emits
  `HirPat::Range { start, end, inclusive, span }`.
- Type-infer: `generate.rs:694-697` — **deferred, no constraint**. Range patterns are
  validated later (not yet fully wired to infer the scrutinee type from the range).
- MIR: decision-tree range check.
- Gotchas: the pattern does not currently constrain the scrutinee type — so
  `match x { 1..5 => ... }` with `x: String` won't produce a type mismatch from the
  pattern itself, only from the scrutinee's other uses.

### AstPat::Enum

- Surface: `.Case`, `.Case(x)`, `.Case(label: x)`.
- CST: `EnumPattern` (`lower.rs:1334`).
- AST-builder: `lower.rs:1522` (`lower_enum_pattern`). Args are `EnumPatternArg` nodes
  with optional labels (via `extract_pattern_arg_label`).
- HIR lowering: `pat.rs:138-142` → `lower_enum_pat` at `pat.rs:243`. Resolves
  `case_name` via `ResolveValuePath`:
  - `ValueResolution::Def(entity)` with `NodeKind::EnumCase` → `HirPat::Variant { entity, args, span }`.
  - anything else (found but not EnumCase, not found, ambiguous) →
    `HirPat::ImplicitVariant { name, args, span }` — left for type inference to
    resolve against the scrutinee type.
- Type-infer: `Variant` → `gen_variant_pat` (`generate.rs:671`); `ImplicitVariant` →
  `gen_implicit_variant_pat` + `Constraint::ImplicitPat` (`generate.rs:675`).
- Solver: `solve_implicit_pat` (2293).
- MIR: decision-tree variant discriminant check + payload bindings.
- Gotchas: the name resolution is just `ResolveValuePath(case_name)` — a single
  segment. Qualified cases like `MyEnum.caseA` don't currently parse as an EnumPattern
  (see `lower_enum_pattern` at 1522 — it only grabs the first Identifier token).

### AstPat::Struct

- Surface: `Point { x, y }`, `Point { x: 0, y }`, `Point { x, .. }`.
- CST: `StructPattern` (`lower.rs:1335`).
- AST-builder: `lower.rs:1593` (`lower_struct_pattern`). Fields are
  `StructPatternField` — shorthand `{ x }` has `pattern: None`, explicit `{ x: p }`
  has `Some(p)`. `has_rest` detected by presence of `StructPatternRest`.
- HIR lowering: `pat.rs:144-149` → `lower_struct_pat` at `pat.rs:299`. Resolves the
  struct name via `ResolveTypePath`:
  - `TypeResolution::Found(entity)` → `HirPat::Struct { entity, fields, has_rest,
    span }`. Validates field names against the struct's actual fields — unknown fields
    are diagnostic; missing fields (without `..`) are diagnostic.
  - Not found → `HirPat::Error { span }`.
  Shorthand fields (`{ x }`) synthesize a `HirPat::Binding` for `x` in the lowered
  output (`pat.rs:313-321`).
- Type-infer: `generate.rs:679-685` — `gen_struct_pat` (elsewhere in generate.rs). Must
  equate scrutinee with `Named(entity, fresh_args)` and recurse into each field.
- MIR: decision-tree field projections.
- Gotchas: shorthand `{ x }` binds `x` to the field value — the HIR pat has a
  `HirStructPatField { field_name: "x", pattern: Some(HirPat::Binding(x_local)) }`.
  This is NOT an empty pattern.

### AstPat::Array

- Surface: `[a, b]`, `[a, .., b]`, `[a, ..name, b]`, `[.., b]`.
- CST: `ArrayPattern` (`lower.rs:1336`).
- AST-builder: `lower.rs:1639` (`lower_array_pattern`). `rest: Option<Option<String>>`
  encodes: `None` (no rest), `Some(None)` (bare `..`), `Some(Some(name))` (named
  `..name` binding).
- HIR lowering: `pat.rs:151-180`. Lowers prefix/suffix patterns. Maps rest:
  - `None` → `None`.
  - `Some(None)` → `Some(None)`.
  - `Some(Some(name))` → `Some(Some(local))` via `define_local(name, force_mut,
    span)` — the rest binding inherits outer `var`.
  Emits `HirPat::Array { prefix, rest, suffix, span }`.
- Type-infer: `generate.rs:710-770`. Handles both `Array[T]` and `Slice[T]`
  scrutinees — if already resolved to `Slice[T]`, reuses the element type; otherwise
  emits `Array[elem_tv]` equate. Equates each prefix/suffix element pattern against
  `elem_tv`. Named rest binding → `Slice[elem_tv]` local.
- Solver: `solve_equal` + `solve_member` (for underlying element projection).
- MIR: decision-tree length check + element projections. See MEMORY
  `array_rest_pattern_port.md` — MIR witness-call port still TODO.

### AstPat::At

- Surface: `name @ subpattern`, `var name @ subpattern`.
- CST: `AtPattern` (`lower.rs:1337`).
- AST-builder: `lower.rs:1688` (`lower_at_pattern`).
- HIR lowering: `pat.rs:182-220`. **Nested `@` patterns are invalid** — emit diagnostic
  and replace the subpattern with `HirPat::Error` so exhaustiveness skips the arm
  instead of seeing an irrefutable `@`-over-wildcard (`pat.rs:188-211`). Regular path
  emits `HirPat::At { binding: local, subpattern, span }`.
- Type-infer: `generate.rs:700-708` — bind local to scrutinee TyVar, recurse into
  subpattern with the same scrutinee.
- MIR: decision-tree runs the subpattern; the binding gets an `Assign` on match.

### AstPat::Or

- Surface: `A | B | C`.
- CST: `OrPattern` (`lower.rs:1338`).
- AST-builder: `lower.rs:1716` (`lower_or_pattern`).
- HIR lowering: `pat.rs:222-230` — recursively lower alternatives, emit
  `HirPat::Or { alternatives, span }`.
- Type-infer: `generate.rs:688-691` — `gen_pat(alt, scrutinee_tv, source)` for each
  alternative. Each alt constrains the same scrutinee.
- MIR: decision-tree union of each alternative's decision.
- Gotchas: all alternatives must bind **the same** locals with the same types — not
  currently enforced by the HIR lowering.

### AstPat::Rest

- Surface: `..` (inside Tuple or Array patterns only).
- CST: `RestPattern` (`lower.rs:1339`).
- AST-builder: `lower.rs:1341` — direct alloc of `AstPat::Rest { span }`.
- HIR lowering: `pat.rs:233-236` — **standalone `Rest` is invalid** — lowered to
  `HirPat::Error { span }`. The valid uses are absorbed by `lower_tuple_pattern`
  and `lower_array_pattern` before reaching `lower_pat_inner`, so when it reaches
  `lower_pat_inner` it means the parser accepted a `..` somewhere it shouldn't be.
- Type-infer: not reachable (lowered to Error before gen_pat sees it).
- MIR: not reachable.
- Gotchas: this variant exists so the AST can represent the token faithfully; it's
  consumed structurally by parent patterns.

### AstPat::Error

- Surface: none — parse error recovery.
- CST: `ErrorPattern` (`lower.rs:1343`) or any unrecognized pattern kind
  (`lower.rs:1347`). Also emitted from many fallback sites:
  `lower.rs:199, 1039, 1154, 1268, 1345, 1349, 1655, 1706, 1767`.
- AST-builder: direct alloc of `AstPat::Error { span }`.
- HIR lowering: `pat.rs:238` → `HirPat::Error { span }`.
- Type-infer: `generate.rs:773` — swallowed (no constraint, no binding).
- MIR: decision-tree treats as unreachable.

---

## HirPat variants (12)

Enum: `lib2/kestrel-hir/src/body.rs:260`. Header comment says "10 variants" — stale,
actually 12.

### HirPat::Wildcard

- Produced by: `AstPat::Wildcard` (`pat.rs:35`). Also synthesized in
  `lower_if_conditions` as the catch-all for let-condition desugaring
  (`expr.rs:1107`).
- Type-infer: `generate.rs:615-617`.
- MIR: nothing emitted; decision tree absorbs.

### HirPat::Binding

- Produced by: `AstPat::Binding` (`pat.rs:37-43`); struct field shorthand
  (`pat.rs:313-321` / `pat.rs:423-427` for ParamPattern); closure param binding
  (`expr.rs:1168, 1199` via HirClosureParam — the HirClosureParam's `pattern` field
  may point at a `HirPat::Binding` in the desugared cases); try-expr / unwrap
  bindings (`desugar.rs:532-535` for `$try_value`, `549-552` for `$try_early`,
  `649-653` for `$unwrap`).
- Type-infer: `generate.rs:619-622` — bind local to scrutinee.
- MIR: `Assign` to `Place::local(binding)`.

### HirPat::Tuple

- Produced by: `AstPat::Tuple` (`pat.rs:72-77`). Also from `ParamPattern::Tuple`
  (`pat.rs:429-439`) — a fn/closure param written as `(a, b): (Int, Int)`.
- Type-infer: `generate.rs:629-668` (see AstPat::Tuple entry).

### HirPat::Literal

- Produced by: `AstPat::Literal` (`pat.rs:80-86`).
- Type-infer: `generate.rs:624-627`.

### HirPat::Range

- Produced by: `AstPat::Range` (`pat.rs:88-136`).
- Type-infer: `generate.rs:694-697` — no constraint yet.

### HirPat::Variant

- Produced by: `AstPat::Enum` that resolved to a `Def(EnumCase)` (`pat.rs:273-277`).
- Type-infer: `generate.rs:671-672` → `gen_variant_pat` — binds payload TyVars to the
  case's declared payload types (substituted with scrutinee's type args) and equates
  scrutinee with the enum's `Named`.
- Gotchas: a qualified `Some(x)` resolves to `Variant(stdOptional.Some, ...)` if it
  resolves at all — otherwise it becomes `ImplicitVariant("Some", ...)`. The lowering
  step decides based on `ResolveValuePath`.

### HirPat::ImplicitVariant

- Produced by: `AstPat::Enum` that did NOT resolve to a `Def(EnumCase)`
  (`pat.rs:279-293`). Also synthesized in desugaring:
  - for-loop `.Some(pattern)` / `.None` arms (`desugar.rs:401-408, 423-427`).
  - try-expr `.Continue($value)` / `.Break($early)` arms (`desugar.rs:536-542, 553-559`).
  - unwrap `.Some($v)` / `.None` arms (`desugar.rs:654-660, 665-668`).
- Type-infer: `generate.rs:675-676` → `gen_implicit_variant_pat` +
  `Constraint::ImplicitPat`.
- Solver: `solve_implicit_pat` (2293) — resolves `.Name` against the scrutinee type's
  enum cases.

### HirPat::Struct

- Produced by: `AstPat::Struct` with resolvable type (`pat.rs:397-402`) or
  `ParamPattern::Struct` (`pat.rs:463-468`).
- Type-infer: `generate.rs:679-685` → `gen_struct_pat`.

### HirPat::Array

- Produced by: `AstPat::Array` (`pat.rs:174-179`). The `rest:
  Option<Option<LocalId>>` encodes: `None` (no rest), `Some(None)` (bare rest),
  `Some(Some(local))` (named rest bound to `Slice[T]`).
- Type-infer: `generate.rs:710-770` (see AstPat::Array entry).

### HirPat::Or

- Produced by: `AstPat::Or` (`pat.rs:227-230`).
- Type-infer: `generate.rs:688-691` — recurse over alternatives with same
  `scrutinee_tv`.

### HirPat::At

- Produced by: `AstPat::At` (`pat.rs:215-219`). Note: nested `@` emits
  `HirPat::At { binding, subpattern: HirPat::Error, .. }` (`pat.rs:203-210`) so
  arm-body references still resolve but exhaustiveness skips the arm.
- Type-infer: `generate.rs:700-708` — bind local, recurse.

### HirPat::Error

- Produced by: `AstPat::Rest` standalone (`pat.rs:235`), `AstPat::Error`
  (`pat.rs:238`), nested-`@` subpattern replacement (`pat.rs:205`), unresolved struct
  name (`pat.rs:404` and `pat.rs:469`).
- Type-infer: `generate.rs:773` — swallow (no constraint).
- Gotchas: `HirPat::Error` is a concrete variant — analyzers and the decision-tree
  builder must handle it.

---

## Sub-references

### `HirLiteral` (used in `HirPat::Literal` and `HirPat::Range`)

Enum: `lib2/kestrel-hir/src/body.rs:334`.

Variants: `Integer(i64)`, `Float(f64)`, `String { value, escape_errors }`, `Char(u32)`,
`Bool(bool)`, `Null`.

Parse helpers in `lib2/kestrel-hir-lower/src/pat.rs`:

- `parse_int` (501) — handles `0x` / `0o` / `0b` radix; falls back to `u64` for values
  above `i64::MAX` so `UInt64.maxValue` round-trips. See MEMORY
  `integer_literal_overflow_silent_zero.md`.
- `parse_float` (518).
- `parse_char` (524) / `parse_char_validated` (535) — `\n\r\t\\\'\"\0\xNN\u{...}`.

### `HirPatArg`

`body.rs:441` — `{ label: Option<String>, pattern: HirPatId }`. Used by `Variant` and
`ImplicitVariant` payloads.

### `HirStructPatField`

`body.rs:448` — `{ field_name: String, pattern: Option<HirPatId> }`. `pattern: None`
never appears in HIR — shorthand `{ x }` is expanded to
`Some(HirPat::Binding(x_local))` during lowering (`pat.rs:313-321`).

---

## Cross-references

- `MatchSource` values control pattern-analyzer gating — see `desugarings.md` and
  `match_pattern_analyzer.md`.
- Cascading pattern-infer errors (TupleRestPat / ImplicitPat arg poisoning) —
  `cascading_infer_errors.md`.
- Array rest MIR port status — `array_rest_pattern_port.md`.
