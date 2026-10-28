# Patterns — CST → HIR → infer → MIR

Covers every `Pat` syntax kind and `HirPat` (12). Verify before citing —
pipeline maps go stale.

Top-level dispatch anchors:

- HIR lowering switch: `lib/kestrel-hir-lower/src/pat.rs` (`lower_pat_inner` →
  `lower_pat_node`)
- Inference gen switch: `lib/kestrel-type-infer/src/generate.rs:607` (`gen_pat`, takes
  a `scrutinee_tv: TyVar` and a `source: MatchSource`)
- MIR lowering: patterns are consumed inside `lower_match` at
  `lib/kestrel-mir-lower/src/body_lower.rs:3319`; there is no stand-alone
  pattern-lowering dispatch.

Patterns never appear in MIR directly — they're compiled into a decision tree by the
`kestrel-pattern-matching` crate, then lowered as `Switch` terminators + binding
assignments.

---

## Pattern syntax → HIR (every `Pat` kind)

Patterns lower from the CST in `lib/kestrel-hir-lower/src/pat.rs` (`lower_pat` →
`lower_pat_inner` → `lower_pat_node`, matching on the `SyntaxKind`). Every lowered pattern
node and every binding identifier is recorded in the body's `BodySourceMap`.

### `WildcardPattern`

- Surface: `_`.
- CST: `_`.
- HIR lowering: `pat.rs` `lower_pat_node` → `HirPat::Wildcard { span }` (1:1).
- Type-infer: `generate.rs:615-617` — no constraint.
- MIR: consumed by decision tree as "matches anything"; emits no binding.
- Gotchas: `let _ = expr;` routes through the complex-pattern path in
  `lower_let_stmt` (not simple binding) because `WildcardPattern` is not
  `BindingPattern`.

### `BindingPattern` / `RefBindingPattern`

- Surface: `x`, `var x`, `mut x` (binding with mut flag).
- CST: `var? ident` / `& mutating? ident`.
- HIR lowering: `pat.rs` `lower_pat_node` — `define_named_local(node, ident, is_mut || force_mut, span)` (records
  the identifier in the source map) then `HirPat::Binding { local, by_ref, span }`.
  `force_mut` propagates an outer `var (a, b) = …` into sub-bindings. A `&` binder
  outside a match arm → E211 and degrades to a plain binding (`by_ref: None`). A binder
  whose name the parser could not find → `HirPat::Error`.
- Type-infer: `generate.rs:619-622` — `ctx.local_types.insert(local, scrutinee_tv)`.
- MIR: decision tree emits an `Assign` to `Place::local(binding)` with the matched
  value.
- Gotchas: `var x` binding sets `is_mut` on the local; `x` alone does not — parameter
  label rules don't apply here, these are binding names.

### `TuplePattern`

- Surface: `(a, b)`, `(a, .., b)`, `(.., b)`, `(a, ..)`.
- CST: `( TuplePatternElement (, TuplePatternElement)* )` (`syntax::tuple_pattern_elements`,
  with rest markers). A one-element `(pat)` without a rest is **grouping**, not a
  1-tuple — `PatSrc::resolve` unwraps it before lowering.
- HIR lowering: `pat.rs` `lower_pat_node`. More than one `..` → E317. Splits at the first rest into
  `prefix` / `suffix` (a second rest lowers to an error inside the suffix) and emits
  `HirPat::Tuple { prefix, has_rest, suffix, span }`.
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

### `LiteralPattern`

- Surface: `5`, `"text"`, `true`, `'c'`.
- CST: one `int` / `float` / `string` / `bool` / `char` token.
- HIR lowering: `pat.rs` `lower_pat_node` — `lower_lit_pat` converts the token to a `HirLiteral` (strings and
  chars through the shared escape table, errors as data). Emits `HirPat::Literal { value, span }`.
- Type-infer: `generate.rs:624-627` — `literal_to_tyvar(value)` + `ctx.equal(lit_tv,
  scrutinee_tv)`.
- Solver: `solve_equal`.
- MIR: decision-tree equality check; see `match_int64_aggregate_ptr_bug.md` for a
  historical bug where Int64 / UInt64 literal patterns compared the scrutinee pointer
  instead of the value.

### `RangePattern`

- Surface: `1..5`, `1..=10`, `'a'..='z'`, `..5`, `0..`.
- CST: literal tokens around `..` / `..=` / `..<`.
- HIR lowering: `pat.rs` `lower_range_pat`. Lowers bounds to `HirLiteral`s and validates
  integer / char ranges (`start <= end`, `<` for exclusive) — E318 otherwise. Emits
  `HirPat::Range { start, end, inclusive, span }`.
- Type-infer: `generate.rs:694-697` — **deferred, no constraint**. Range patterns are
  validated later (not yet fully wired to infer the scrutinee type from the range).
- MIR: decision-tree range check.
- Gotchas: the pattern does not currently constrain the scrutinee type — so
  `match x { 1..5 => ... }` with `x: String` won't produce a type mismatch from the
  pattern itself, only from the scrutinee's other uses.

### `EnumPattern` / `NullPattern` / `SomePattern`

- Surface: `.Case`, `.Case(x)`, `.Case(label: x)`.
- CST: `. ident ( EnumPatternArg, … )?`; `null` is `.None`, `some p` is `.Some(p)`. An
  argument that is a bare identifier binds it (`PatSrc::ArgBinding`).
- HIR lowering: `pat.rs` `lower_enum_pat`. Lowers the arguments, then resolves the case name
  via `ResolveValuePath`:
  - `ValueResolution::Def(entity)` with `NodeKind::EnumCase` → `HirPat::Variant { entity, args, span }`.
  - anything else (found but not EnumCase, not found, ambiguous) or a missing name →
    `HirPat::ImplicitVariant { name, args, span }` — left for type inference to
    resolve against the scrutinee type.
- Type-infer: `Variant` → `gen_variant_pat` (`generate.rs:671`); `ImplicitVariant` →
  `gen_implicit_variant_pat` + `Constraint::ImplicitPat` (`generate.rs:675`).
- Solver: `solve_implicit_pat` (2293).
- MIR: decision-tree variant discriminant check + payload bindings.
- Gotchas: the name resolution is just `ResolveValuePath(case_name)` — a single
  segment. Qualified cases like `MyEnum.caseA` don't currently parse as an EnumPattern
  (see `lower_enum_pattern` at 1522 — it only grabs the first Identifier token).

### `StructPattern`

- Surface: `Point { x, y }`, `Point { x: 0, y }`, `Point { x, .. }`.
- CST: `ident { StructPatternField (, …)*, ..? }` — shorthand `{ x }` has no pattern.
- HIR lowering: `pat.rs` `lower_struct_pat`. Resolves the struct name via `ResolveTypePath`:
  - `TypeResolution::Found(entity)` → `HirPat::Struct { entity, fields, has_rest, span }`;
    unknown fields → E320, uncovered fields without `..` → E321.
  - Not found (or no name) → `HirPat::Error { span }`.
  Shorthand fields (`{ x }`) synthesize a `HirPat::Binding` for `x` (not recorded as a
  named declaration in the source map — the token is also the field name).
- Type-infer: `generate.rs:679-685` — `gen_struct_pat` (elsewhere in generate.rs). Must
  equate scrutinee with `Named(entity, fresh_args)` and recurse into each field.
- MIR: decision-tree field projections.
- Gotchas: shorthand `{ x }` binds `x` to the field value — the HIR pat has a
  `HirStructPatField { field_name: "x", pattern: Some(HirPat::Binding(x_local)) }`.
  This is NOT an empty pattern.

### `ArrayPattern`

- Surface: `[a, b]`, `[a, .., b]`, `[a, ..name, b]`, `[.., b]`.
- CST: `[ (ArrayPatternElement | ArrayPatternRest), … ]`; `ArrayPatternRest` is `.. ident?`.
- HIR lowering: `pat.rs` `lower_pat_node`. Lowers prefix, defines the rest binding (named `..name` →
  `Some(Some(local))`, inheriting an outer `var`; bare `..` → `Some(None)`; none →
  `None`), lowers suffix. Emits `HirPat::Array { prefix, rest, suffix, span }`.
- Type-infer: `generate.rs:710-770`. Handles both `Array[T]` and `Slice[T]`
  scrutinees — if already resolved to `Slice[T]`, reuses the element type; otherwise
  emits `Array[elem_tv]` equate. Equates each prefix/suffix element pattern against
  `elem_tv`. Named rest binding → `Slice[elem_tv]` local.
- Solver: `solve_equal` + `solve_member` (for underlying element projection).
- MIR: decision-tree length check + element projections. See MEMORY
  `array_rest_pattern_port.md` — MIR witness-call port still TODO.

### `AtPattern`

- Surface: `name @ subpattern`, `var name @ subpattern`.
- CST: `var? ident @ Pat`.
- HIR lowering: `pat.rs` `lower_pat_node`. **Nested `@` is invalid** — E319, and the subpattern is replaced by
  `HirPat::Error` so exhaustiveness skips the arm instead of seeing an irrefutable
  `@`-over-wildcard. Otherwise `HirPat::At { binding: local, subpattern, span }`.
- Type-infer: `generate.rs:700-708` — bind local to scrutinee TyVar, recurse into
  subpattern with the same scrutinee.
- MIR: decision-tree runs the subpattern; the binding gets an `Assign` on match.

### `OrPattern`

- Surface: `A | B | C`.
- CST: `Pat or Pat (or Pat)*`.
- HIR lowering: `pat.rs` `lower_pat_node` — lowers the first alternative, then every later alternative
  **reuses** the locals it bound (`set_or_reuse`, #187), so all alternatives and the arm
  body share one local per name; emits `HirPat::Or { alternatives, span }`.
- Type-infer: `generate.rs:688-691` — `gen_pat(alt, scrutinee_tv, source)` for each
  alternative. Each alt constrains the same scrutinee.
- MIR: decision-tree union of each alternative's decision.
- Gotchas: all alternatives must bind **the same** locals with the same types — not
  currently enforced by the HIR lowering.

### `RestPattern` (standalone)

- Surface: `..` (inside Tuple or Array patterns only).
- CST: `..` outside a tuple/array element list.
- HIR lowering: `pat.rs` `lower_pat_node` — **standalone rest is invalid** → `HirPat::Error { span }`. Valid
  uses are consumed by the tuple/array arms.
- Type-infer: not reachable (lowered to Error before gen_pat sees it).
- MIR: not reachable.
- Gotchas: the token is consumed structurally by the parent tuple/array pattern.

### Malformed / missing patterns

- Surface: none — parse error recovery.
- CST: a missing pattern child, `ErrorPattern`, or any unrecognised kind.
- HIR lowering: `PatSrc::Error(span)` / the `_` arm of `lower_pat_node` → `HirPat::Error { span }`.
- Type-infer: `generate.rs:773` — swallowed (no constraint, no binding).
- MIR: decision-tree treats as unreachable.

---

## HirPat variants (12)

Enum: `lib/kestrel-hir/src/body.rs:260`. Header comment says "10 variants" — stale,
actually 12.

### HirPat::Wildcard

- Produced by: `WildcardPattern` (`pat.rs`). Also synthesized in
  `lower_if_conditions` as the catch-all for let-condition desugaring
  (`expr.rs`).
- Type-infer: `generate.rs:615-617`.
- MIR: nothing emitted; decision tree absorbs.

### HirPat::Binding

- Produced by: `BindingPattern` (`pat.rs`); struct field shorthand
  (`pat.rs` / `pat.rs` for ParamPattern); closure param binding
  (`expr.rs` via HirClosureParam — the HirClosureParam's `pattern` field
  may point at a `HirPat::Binding` in the desugared cases); try-expr / unwrap
  bindings (`desugar.rs` for `$try_value`, `549-552` for `$try_early`,
  `649-653` for `$unwrap`).
- Type-infer: `generate.rs:619-622` — bind local to scrutinee.
- MIR: `Assign` to `Place::local(binding)`.

### HirPat::Tuple

- Produced by: `TuplePattern` (`pat.rs`). Also from `ParamPattern::Tuple`
  (`pat.rs`) — a fn/closure param written as `(a, b): (Int, Int)`.
- Type-infer: `generate.rs:629-668` (see TuplePattern entry).

### HirPat::Literal

- Produced by: `LiteralPattern` (`pat.rs`).
- Type-infer: `generate.rs:624-627`.

### HirPat::Range

- Produced by: `RangePattern` (`pat.rs`).
- Type-infer: `generate.rs:694-697` — no constraint yet.

### HirPat::Variant

- Produced by: `EnumPattern` that resolved to a `Def(EnumCase)` (`pat.rs`).
- Type-infer: `generate.rs:671-672` → `gen_variant_pat` — binds payload TyVars to the
  case's declared payload types (substituted with scrutinee's type args) and equates
  scrutinee with the enum's `Named`.
- Gotchas: a qualified `Some(x)` resolves to `Variant(stdOptional.Some, ...)` if it
  resolves at all — otherwise it becomes `ImplicitVariant("Some", ...)`. The lowering
  step decides based on `ResolveValuePath`.

### HirPat::ImplicitVariant

- Produced by: `EnumPattern` that did NOT resolve to a `Def(EnumCase)`
  (`pat.rs`). Also synthesized in desugaring:
  - for-loop `.Some(pattern)` / `.None` arms (`desugar.rs`).
  - try-expr `.Continue($value)` / `.Break($early)` arms (`desugar.rs`).
  - unwrap `.Some($v)` / `.None` arms (`desugar.rs`).
- Type-infer: `generate.rs:675-676` → `gen_implicit_variant_pat` +
  `Constraint::ImplicitPat`.
- Solver: `solve_implicit_pat` (2293) — resolves `.Name` against the scrutinee type's
  enum cases.

### HirPat::Struct

- Produced by: `StructPattern` with resolvable type (`pat.rs`) or
  `ParamPattern::Struct` (`pat.rs`).
- Type-infer: `generate.rs:679-685` → `gen_struct_pat`.

### HirPat::Array

- Produced by: `ArrayPattern` (`pat.rs`). The `rest:
  Option<Option<LocalId>>` encodes: `None` (no rest), `Some(None)` (bare rest),
  `Some(Some(local))` (named rest bound to `Slice[T]`).
- Type-infer: `generate.rs:710-770` (see ArrayPattern entry).

### HirPat::Or

- Produced by: `OrPattern` (`pat.rs`).
- Type-infer: `generate.rs:688-691` — recurse over alternatives with same
  `scrutinee_tv`.

### HirPat::At

- Produced by: `AtPattern` (`pat.rs`). Note: nested `@` emits
  `HirPat::At { binding, subpattern: HirPat::Error, .. }` (`pat.rs`) so
  arm-body references still resolve but exhaustiveness skips the arm.
- Type-infer: `generate.rs:700-708` — bind local, recurse.

### HirPat::Error

- Produced by: `RestPattern` standalone (`pat.rs`), `malformed pattern`
  (`pat.rs`), nested-`@` subpattern replacement (`pat.rs`), unresolved struct
  name (`pat.rs` and `pat.rs`).
- Type-infer: `generate.rs:773` — swallow (no constraint).
- Gotchas: `HirPat::Error` is a concrete variant — analyzers and the decision-tree
  builder must handle it.

---

## Sub-references

### `HirLiteral` (used in `HirPat::Literal` and `HirPat::Range`)

Enum: `lib/kestrel-hir/src/body.rs:334`.

Variants: `Integer(i64)`, `Float(f64)`, `String { value, escape_errors }`, `Char(u32)`,
`Bool(bool)`, `Null`.

Parse helpers in `lib/kestrel-hir-lower/src/pat.rs`:

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
`Some(HirPat::Binding(x_local))` during lowering (`pat.rs`).

---

## Cross-references

- `MatchSource` values control pattern-analyzer gating — see `desugarings.md` and
  `match_pattern_analyzer.md`.
- Cascading pattern-infer errors (TupleRestPat / ImplicitPat arg poisoning) —
  `cascading_infer_errors.md`.
- Array rest MIR port status — `array_rest_pattern_port.md`.
