# kestrel-hir-lower — Agent Guide

Patterns and invariants for lowering CST → HIR (the desugaring layer).
Read this before adding or changing a desugaring in `desugar.rs` / `expr.rs`.

## Operators desugar to a `ProtocolCall` — never hand-roll a match

Every operator lowers to a `HirExpr::ProtocolCall` whose protocol entity comes
from a builtin (`Builtin::*OperatorProtocol`) resolved via `resolve_builtin`,
and whose semantics live in the stdlib protocol's conformances — **not** in a
match/branch tree synthesized here.

- Binary ops → `lookup_binary_op` / `lookup_short_circuit_op`
- Unary ops → `lookup_unary_op`
- Postfix ops (`..`, `!`) → `lookup_postfix_op` + `desugar_postfix_op`
- Compound assign (`+=`, …) → `lookup_compound_assign_op` + `desugar_compound_assign`

Adding (or fixing) an operator means: define a `Protocol` + method in the
stdlib with `@builtin(.XOperatorProtocol)` / `@builtin(.XOperatorMethod)`
(model on `lang/std/core/coalesce.ks`), register the two `Builtin` variants in
`kestrel-hir/src/builtin.rs` (enum + `name()` + `from_str` + `kind()`), add a
row to the relevant lookup table in `kestrel-hir/src/body.rs`, and route the
syntax kind through the matching `desugar_*` helper. The conformance carries
the behavior; the lowerer only emits the call.

**Why this is a hard rule:** a hand-rolled match has to invent the type of every
arm, and the lowerer has no real type information. Postfix `!` originally
hand-rolled `match opt { .Some(v) => v, .None => HirExpr::Error }`; the
`HirExpr::Error` arm got the poison `Error` type (not `Never`) and lowered to a
*value* that didn't terminate its block, so it fed a type-mismatched block-arg
into the match merge and **ICE'd at MIR OSSA verify**. Routing it through
`ForceUnwrap.forceUnwrap()` (whose `.None` arm is `fatalError(...) -> !` inside
the stdlib) made the divergence a real `Panic` terminator and the merge type
fall out as the protocol's `Output`. A `ProtocolCall` returns a single,
inference-resolved type — there is no merge to get wrong.

Corollary: if a desugaring genuinely needs a divergent/trapping arm, emit a
call to a `-> !` (Never) stdlib function (e.g. `fatalError`) or an existing
diverging `HirExpr` (`Return`), never `HirExpr::Error`. `HirExpr::Error` is for
error *recovery* (poison that suppresses cascades), not for "unreachable".

## `HirExpr::Error` is recovery-only

Reserve `HirExpr::Error` for parse/resolve failures where you want downstream
inference to absorb the node silently. It is **not** a trap, a unit value, or a
"fill in later" placeholder — it carries the poison `Error` type and lowers to a
non-terminating dummy value, which breaks any context that expects a real type
(branch merges, returns). When recovering inside a desugaring, wrap the
`HirExpr::Error` in the same `Sugar`/diagnostic shape the success path uses (see
`desugar_try` / `desugar_compound_assign`) so consumers see a uniform node.

## Missing syntax lowers to an explicit error form — never `""`

An absent child in the CST (a recovered parse) lowers to `ExprSrc::Error` →
`HirExpr::Error`, `PatSrc::Error` → `HirPat::Error`, or `HirName::Missing` for
an absent member/case name. Never synthesize an empty-string name: `""` looks
like a real identifier to every later stage (name resolution, the solver, the
LSP) and produces nonsense diagnostics instead of a silent, already-reported
gap. This is the recovery case of the rule above.

## HIR ids are allocated in source order

Allocate expression/pattern/statement ids in the order their syntax appears.
Diagnostic order, `while_conditions`, and several analyzers walk the arenas in
id order and assume it matches the source. When a desugaring needs a node
before its operands exist, still lower the operands first.

## Record every user-spelled binding and path segment in the source map

`BodySourceMap` is the only way tools get from a position to HIR and back
(`src/source_map.rs`). Define a local the user wrote with
`define_named_local` (binding node + identifier), and record each path segment
with `alloc_seg` / `record_segments`. A local with no single declaring
identifier — a desugaring temporary, `self`, an implicit `it`, a struct-pattern
shorthand — stays UNRECORDED on purpose, so rename/references fail closed
instead of guessing (audit F2).

## A member named through a type is a member OF that type

`Type[Args].member`, `Type.member` and `Self.member` lower with the **type as
the receiver** (`lower_type_prefix` in `src/expr.rs`), never as a `Def` of the
member carrying one flat `[owner…, own…]` type-arg list:

- a static call becomes a `MethodCall` on the type;
- a computed static property or an unapplied static method becomes a `Field`
  on the type;
- an enum case becomes `ImplicitMember { owner: Some(type) }`.

The type's `[args]` bind the type's params, and the member's own `[args]` bind
the member's. A bare `Type` is `Type[_]`, inferred per use, even inside
`Type`'s own body. `Self` is the enclosing instantiation. A flat list has no
answer for `HMAC.setup(..)` inside `HMAC[H]`, and the backend used to guess
from the raw brackets.

**Exception: a stored `static var` stays `Def(field)`.** It is one global for
every instantiation, so it has no type args. Every MIR global place and assign
path stores through `Def`.

Also, `ResolveValuePath` returns `Def(case)` for a case in the **last**
segment, and `EnumCaseValue` only when more segments follow. Check both.

## A protocol is a type only through `any` / `some` (E144)

`lower_ast_type` is the **value-position** entry point: a path that resolves
to a protocol there is E144 (`reject_bare_protocol`), never `HirTy::Protocol`.
A position that names the protocol *itself* lowers through
`lower_bound_type` instead, which resolves the same path without E144:

- `some P` and `any P` bounds,
- `extend P` targets (`lower_self_hir_ty`),
- the `P` of a qualified `P.Item` (`build_assoc_projection_base`).

When you add a new place that names a protocol (a new bound syntax, a new
qualified form), route it through `lower_bound_type`. If it goes through
`lower_ast_type`, every stdlib `extend Iterator { … }` style use starts
failing with E144 (the M1 work hit both of those cases).
Unused declarations never get lowered, so `type_annotation_resolution`
forces the lowering when an annotation names a protocol in a value position
(the same gate it uses for reference types).
