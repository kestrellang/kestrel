# kestrel-hir-lower — Agent Guide

Patterns and invariants for lowering AST → HIR (the desugaring layer).
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
AST variant through the matching `desugar_*` helper. The conformance carries
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
