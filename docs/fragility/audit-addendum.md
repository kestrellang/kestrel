# Addendum — Gap Round & High-Severity Confirmation

*Companion to `temp/audit/fragility-audit-2026-08-08.md` (90 findings). This document does not restate the main report; it settles the two findings that report ranked #1 and #2, adds 17 findings from three under-covered axes, and states honestly what neither pass has looked at. Every line number below was re-read in the working tree at commit `932f2ba9` — note that `lib/kestrel-mir/src/mono/mod.rs`, `lib/kestrel-mir/src/ty_query.rs`, `lib/kestrel-mir-lower/src/body/control.rs` and `lib/kestrel-codegen-cranelift/src/inst.rs` are **modified but uncommitted** by concurrent work, so several line numbers here differ by a few lines from what an auditor reading the last commit would have seen.*

---

## High-Severity Findings: Independent Verdicts

### #1 — Range-overlap specialization misroutes match arms → **CONFIRMED-REACHABLE**

The main report did not overstate this. It understated it: the finding says the wrong arm is selected; the truth is that the *entire body of arm 1 becomes dead code for every input*, and there is a second variant with no diagnostic at all.

**Decisive evidence.**

- `Constructor::matches` routes `(IntRange, IntRange)` to `ranges_overlap_i64` at `lib/kestrel-pattern-matching/src/constructor.rs:146` (helper at `:837`) — a non-empty-intersection test, used where specialization needs containment.
- `FlatPat::decompose` gates row retention on it: `if !ctor.matches(target_ctor)` at `lib/kestrel-pattern-matching/src/flat_pat.rs:85`, reached from `PatternMatrix::specialize` (`matrix.rs:142`).
- The correction exists in exactly one consumer and says so verbatim: `lib/kestrel-pattern-matching/src/usefulness.rs:112-116` — *"The usefulness algorithm's `specialize` treats any overlap between two ranges as full coverage, which misclassifies partial overlaps as redundant. We fix that here"* — with `range_covered_by_union_i64` applied out-of-band at `usefulness.rs:138`.
- `decision_tree::compile_matrix` has no counterpart. It calls `matrix.specialize(query, root, col, ctor)` raw at `lib/kestrel-pattern-matching/src/decision_tree.rs:180`, and `compile_leaf` takes `rows.first()` at `decision_tree.rs:221`.
- Nothing downstream rescues it. Both backends emit an ordered first-match chain, not a jump table: `lib/kestrel-codegen-cranelift/src/terminator.rs:300` (`SwitchCase::IntRange` → `range_predicate` → `emit_case_branch`) and `lib/kestrel-codegen-llvm/src/terminator.rs:352`.
- Nothing blocks the build. E307 is `Severity::Warning` (`lib/kestrel-analyze/src/body/exhaustiveness.rs:69`, descriptor id at `:73`), and `src/main.rs:218` aborts only on `has_errors(&compiler) || analyze_summary.errors > 0`.

**Minimal repro.** `lib/kestrel-test-suite/testdata/patterns/exhaustiveness/overlapping_ranges.ks:7-11` is already the exact match (`0..=10 => "first"`, `5..=15 => "second"`, `_ => "other"`), but line 1 is `// test: diagnostics` — it asserts the warning and never runs. Add a driver:

```kestrel
func classify(x: Int64) -> Int64 {
    match x { 0..=10 => 1, 5..=15 => 2, _ => 0 }
}
// classify(12) returns 1, not 2
```

Hand trace: `specialize(IntRange{5,15})` retains the `0..=10` row because `ranges_overlap_i64(0,10,5,15)` is true, and that row is *first*, so `compile_leaf` yields `Success{arm 0}`. The emitted chain is `[0..=10 → arm0body, 5..=15 → arm0body, _ → arm2body]`. `x = 12` fails test 1, passes test 2, runs arm 0.

**What a user experiences.** A compiled program that returns the wrong value, with a yellow squiggle on the arm that *does* work. The warning actively misdirects: it points at arm 1 as "overlapping", when arm 1 is the arm that has been silently deleted.

**Worse variant the main report missed.** `extract_int_range` (`usefulness.rs:133`) inspects only the arm's top-level `FlatPat`. Nested ranges — `(0..=10, _)` vs `(5..=20, _)`, or `.Some(0..=10)` vs `.Some(5..=20)` — never enter `prior_int_ranges`, so **no E307 fires at all**; instead `is_useful`, running on the same overlap-as-containment `specialize`, marks the second arm redundant and emits a spurious E306 "unreachable pattern" (also a Warning, `exhaustiveness.rs:66-71`). Same wrong-arm routing, and the diagnostic now tells the user the correct arm is dead.

**Correction to the main report's fix location.** `lib/kestrel-pattern-matching/AGENTS.md` already states the fix belongs in the matrix layer, not in `check_match`. The `range_covered_by_union_i64` patch in `usefulness.rs` should be *deleted* as part of the fix, not left alongside a second correction — otherwise there are three range rules.

---

### #2 — LSP local rename destroys code → **CONFIRMED-REACHABLE**, and the parameter case is worse than reported

**Decisive evidence.**

- `identifier_for_target` has two arms that disagree. The entity arm narrows to the identifier token: `get_name_span(&cst.0, decl_span.0.file_id)` at `lib/kestrel-lsp/src/handlers/rename.rs:228`. The local arm does not narrow at all: `Some((local.name.clone(), local.span.clone()))` at `rename.rs:238`.
- `push_decl_site` hardcodes `kind: RefKind::Direct` at `rename.rs:260`, and `clip_to_identifier` early-returns the span **unchanged** for `Direct`: `if matches!(kind, RefKind::Direct) { return span.clone(); }` at `lib/kestrel-lsp/src/references.rs:192-194`. So no narrowing happens anywhere on that path.
- `Local.span` is whatever `define_local` was handed (`lib/kestrel-hir-lower/src/ctx.rs:125-138`), and three of the four binder sites hand it something that is not an identifier:
  - `let`/`var`: `lib/kestrel-hir-lower/src/stmt.rs:126` passes `span.clone()`, the `span: &Span` parameter of `lower_let_stmt` (`stmt.rs:71`) — the whole `VariableDeclaration` node, `let` keyword through semicolon.
  - function param: `lib/kestrel-hir-lower/src/lib.rs:79` passes `Span::synthetic(0)`.
  - closure param: `lib/kestrel-hir-lower/src/expr.rs:1361` passes `span.clone()` — the `span: &Span` of `lower_closure` (`expr.rs:1334`), i.e. **the entire closure expression**.
  - match-arm binding (`pat.rs`) is the only correct one.
- The correct span is available and discarded twice: `stmt.rs` and `expr.rs:1348` both match `AstPat::Binding { name, is_mut, .. }`, and the `..` swallows the `span` field. For function params it does not exist — `AstParam` has no span field.
- `prepare_rename` does not mask it: `rename.rs:68-70` calls the identical `identifier_for_target` and hands the raw span to `li.range_for(...)`. It neither narrows nor rejects, so the pre-rename highlight is also the whole statement.
- Live and shipped: `rename_provider` with `prepare_provider: Some(true)` at `lib/kestrel-lsp/src/lib.rs:232-235`; `kestrel-lsp` is staged into releases by `.github/workflows/_release-build.yml:37`.

**Minimal repro and what a user experiences.**

```kestrel
module Test
func compute(width: lang.i64) -> lang.i64 {
  let total = width * 2;
  total + width
}
```

F2 on the `total` in line 4, rename to `sum`. The use site is edited correctly; the decl site edit spans `let total = width * 2;` and replaces all of it with `sum`. The buffer becomes `sum` / `sum + width`. The initializer, the keyword and the semicolon are gone, and the file no longer compiles. It is undoable but not obvious — the user sees two edits, one of which silently ate a line.

F2 on `width` (a parameter): the decl span is `0..0`, so the correct use-site edits are accompanied by an insertion at line 0, char 0 — `w` is spliced into `module Test`, and the parameter itself is never renamed, so the body now references an undeclared name.

**Also found while verifying, not in the main report.** F2 pressed on the *declaration* identifier does not reach the local path at all. `semantic::hir_expr_at` (`lib/kestrel-lsp/src/semantic.rs:89-101`) scans only `body.exprs`, no expression span covers the binder, so `target_at` falls through to `semantic::enclosing_decl_at` and returns the **enclosing function**. The rename widget opens pre-filled with `compute`, and confirming renames the function across the workspace. Loud enough that most users would abort, but it is the same missing-span root cause.

**Verdict on the main report's framing.** Accurate, and its Tier-0 ranking is right — this is the only finding in either pass with irreversible consequences for a user's files. Its "needs a maintainer decision: container or identifier?" note (Tier 3) is the wrong framing though: hover wanting a container span and rename wanting an identifier span is not a conflict to resolve, it is two facts that need two fields. `Local::name_span` is the fix; there is nothing to decide.

---

## New Findings from the Gap Round

The gap round was productive, which is itself a result: the first pass's 166 cited files left three axes almost untouched, and each of them yielded confirmed defects on the first sweep. **All 17 findings below survived adversarial verification; none is high severity.** That is the honest headline — the first pass did find the two worst things in the compiler. What it missed is a layer of medium-severity defects that share two *new* systemic shapes.

### Two systemic themes to add to the main report's five

**Theme 6 — cross-crate invariants are tracked by a numbered registry that lives in a completed plan document.** `docs/plans/closure-kinds/closure-kinds-plan.md:439-458` defines nine "lockstep constraints" — e.g. *"1. The six resource predicates: `copy_behavior` ↔ `needs_drop` ↔ `expand.rs ty_needs_drop` ↔ `audit.rs mono_needs_drop` ↔ `clone_shim.rs ty_needs_clone_shim` ↔ `concrete_copy` catch-all"*, *"9. Owning-kind 4-word layout: `passes/layout.rs` ↔ cranelift `ty.rs` ↔ llvm `ty.rs` ↔ both `compile_apply_partial` pair slots"*. **26 files under `lib/` cite these by number** (`lib/kestrel-mir/src/ty_query.rs:122`, `lib/kestrel-mir/src/passes/layout.rs:118`, `lib/kestrel-codegen-llvm/AGENTS.md:44`, `lib/kestrel-mir/src/mono/mangle.rs:205`, and 22 more). This is the main report's Theme 1 with a twist that makes it worse: the maintainers *know* about the N-way duplication, have numbered it, and put the index in a plan doc for a feature that has already shipped. Nothing in `AGENTS.md` or in code enumerates the nine. A newcomer reading `ty_query.rs:122` can find four of the six sites in the comment; to find the rest they must know that a plan document is load-bearing. Finding A1 below is a live break of lockstep 1; A5 is a live break of lockstep 9.

**Theme 7 — the compilation target is an un-audited compiler input.** There are three unrelated `TargetConfig` types (`lib/kestrel-ast-builder/src/components.rs:337` — `{ os }`; `lib/kestrel-mir/src/item/mod.rs:96` — `{ pointer_width }`; `lib/kestrel-codegen/src/target.rs` — `{ triple, pointer_width }`), sharing no constructor, no trait, and no cross-check. `--target` reaches two of the three (A4). 18 distinct `KESTREL_*` environment variables were found by grep; one of them changes emitted instructions (A17) and none of them is part of any query key.

---

#### A1. Init drop-flag setup reads `needs_drop` at `Stage::Raw` — before `drop_fix` populates it — and the fallback disjunct misses every Cloneable aggregate

**Category** ordering-dependency / **Severity** medium / **Locations**
`lib/kestrel-mir-lower/src/body/mod.rs:1198`, `:1202`, `:1203`, `:887`, `:1245`, `:1247`, `:1363`
`lib/kestrel-mir-lower/src/body/expr.rs:519`
`lib/kestrel-mir-lower/src/items/struct_lower.rs:55`, `:68`
`lib/kestrel-mir/src/passes/drop_fix.rs:18`
`lib/kestrel-mir/src/passes/mod.rs:126`
`lib/kestrel-mir/src/ty_query.rs:338`
`lang/std/text/string.ks:185`

**Evidence.** `drop_fix.rs:3-8` states the contract: MIR lowering sets `DropBehavior` from user `deinit`s only, "returning `None` for types that lack a deinit but contain droppable fields. This pass runs after all types are lowered." That lowering is `lower_drop_behavior` (`struct_lower.rs:60-70`), which returns `DropBehavior::None` at `:68` whenever `find_user_deinit` is `None`, regardless of fields. `ty_query::needs_drop`'s Named arm reads exactly that field: `s.type_info.drop != DropBehavior::None` (`ty_query.rs:338-341`). `passes/mod.rs:126` is the **only** call site of `drop_fix::fix_drop_behaviors` in the repo (verified by grep), and it runs inside the pass pipeline — strictly after body lowering.

But `setup_init_field_flags` calls `needs_drop` *during* lowering, and its own comment (`body/mod.rs:1193-1197`) admits the hazard and offers a compensating disjunct:

```rust
let droppable = kestrel_mir::ty_query::needs_drop(
    &self.ctx.module.ty_arena, &self.ctx.module, field_ty,
) || self.is_non_copyable(field_ty);
```

`is_non_copyable` is `matches!(self.copy_behavior_of(ty), CopyBehavior::None)` (`body/mod.rs:887-889`) — true only for move-only types. `struct_lower.rs:55` maps `CopySemantics::Cloneable => CopyBehavior::Clone(entity)`, so a Cloneable type is false on **both** disjuncts. `lang/std/text/string.ks:185` declares `public struct String: … Cloneable …` and the only `deinit` in that file is on `StringStorage` (`:126`).

The two consumers then no-op: `store_init_self_field` falls to a bare `emit_store_init` on `field_init(...) == None` (`body/mod.rs:1245-1249`), and `emit_init_partial_drops` iterates only `init_field_flags` (`body/mod.rs:1363`), which is empty. The failure-return classifier is also gated on `!self.init_field_flags.is_empty()` (`body/expr.rs:519`), so for an all-Cloneable-field struct the failable-init path is not even recognised.

**Why it matters.** Two places encode "is this field droppable?" — `type_info.drop`, authoritative only after `drop_fix`, and this ad-hoc disjunction at `Stage::Raw`. Because the pass that makes the first one true runs strictly later, the lowering answers a stale question, and the compensating disjunct covers one of three copy classes. This is a live break of lockstep 1. The existing regression fixture is structurally unable to catch it: `lib/kestrel-test-suite/testdata/memory_model/deinit/init_field_reassign_drops_old.ks:23` uses `struct Res: not Copyable { … deinit { … } }`, satisfying both disjuncts.

**Failure scenario.**

```kestrel
struct Holder {
    var s: String
    var n: Int64
    init() {
        self.s = "a heap-allocated string long enough to malloc";
        self.s = "replacement";   // must drop the first buffer — does not
        self.n = 0;
    }
}
```

Field 0 gets no drop flag, so both assignments emit a bare `StoreInit` and the first buffer leaks. No later pass rescues it — only `StoreAssign` gets the destroy-old expansion (`lib/kestrel-mir/src/mono/expand.rs:886`); `StoreInit` is only value-remapped (`expand.rs:1242`). The same field in an `init?` leaks on the failure return. Deterministic heap leak, no diagnostic, no verifier error. It is *not* memory-unsafety — no double free — which is why this is medium and not high.

**Fix.** Make droppability a single post-`drop_fix` fact. `drop_fix::fix_drop_behaviors` only needs `module.structs`/`module.enums`, not bodies, so it can be hoisted ahead of body lowering; alternatively have `setup_init_field_flags` reuse `drop_fix::field_needs_drop`'s structural fixed point rather than its not-yet-written output. Stopgap: widen the fallback to `copy_behavior != Bitwise`. Add an execution test whose field type is a deinit-less `Cloneable` struct.

---

#### A2. The mono layout work-list is seeded only from body *value* types, so a type reachable only through `Op::SizeOf`/`Op::AlignOf` gets no `MonoStruct`, and codegen answers size 8

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-mir/src/mono/mod.rs:805`, `:818`, `:844`, `:849`, `:856`, `:1094`, `:1149`, `:1159`, `:1169`, `:1175`, `:1255`, `:623`
`lib/kestrel-mir/src/mono/verify.rs:51`
`lib/kestrel-codegen-cranelift/src/ty.rs:187`, `src/inst.rs:626`, `:2078`
`lib/kestrel-codegen-llvm/src/ty.rs:259`, `src/inst.rs:728`, `:2121`
`lib/kestrel-mir-lower/src/body/call/intrinsic.rs:1298`
`lang/std/memory/layout.ks:47`

**Evidence.** `resolve_types_and_layouts` builds its entire work-list from `collect_named_types` over mono bodies (`mono/mod.rs:803-807`) and then iterates `&concrete_types` (`:818`) — a borrowed `IndexMap`, so the set provably cannot grow during the fixed point. `collect_named_types` whitelists exactly four instruction kinds — `Struct`, `Enum`, `Array`, `Literal` (`:1149-1166`) — with `_ => {}` at `:1169`. `Op1`/`Op2`/`Op3` are absent, yet `substitute_op_type` (`:623-645`) proves those ops carry first-class monomorphized types (`Op::SizeOf(ty) | Op::AlignOf(ty) | Op::StackAlloc(ty) | Op::Ptr*(ty)`).

What makes this a defect rather than an oversight-in-theory: the same collector **already special-cases** `ImmediateKind::SizeOf/AlignOf/NullPtr` at `:1159-1164`. The authors knew sizeof operands must be collected; they missed the `Op1` spelling of the same thing.

When a type is absent, `mono_size_and_align`'s Named arm is a bare `layout_cache.get(&key).copied()` (`:1255-1258`) → `None` → `all_resolved = false` (`:844`) → the `if all_resolved` guard at `:849` skips the `mono_structs.insert` at `:856` **entirely**. The intended guard cannot fire: `verify_mono` scans `module.structs.values()` for `layout.is_none()` (`verify.rs:50-51`), but every inserted `MonoStruct` gets `layout = Some(...)` at `mono/mod.rs:855`. A struct with an unresolvable layout is *absent*, not layout-less.

Both backends then substitute silently and inconsistently: `classify_named` returns `TypeRepr::Scalar(ptr)` — 8 bytes — at `cranelift/ty.rs:187` and `llvm/ty.rs:259` (cranelift's diagnostic is behind `std::env::var("KESTREL_DEBUG_CLONE")`, `ty.rs:182`); `struct_field_offset` returns literal `0` at `cranelift/inst.rs:2078` and `llvm/inst.rs:2121`; `concrete_copy` answers `.unwrap_or(CopyBehavior::Bitwise)` at `mono/mod.rs:1094`.

**Why it matters.** `Op::SizeOf`/`Op::AlignOf` are the one place a type's size is *used* without a value of that type existing — they are the entire basis of the `Layout` API and the allocator. Excluding them means the compiler can be asked for a struct's size and confidently answer the pointer width, with a wrong `CopyBehavior` for the same type, and nothing reports it.

**Failure scenario.**

```kestrel
struct Vec3 { var x: Int64; var y: Int64; var z: Int64 }   // 24 bytes
let l = Layout.of[Vec3]();
// l.size == 8
```

`Layout.of[T]` (`lang/std/memory/layout.ks:47`) calls `lang.sizeof[T]`, which lowers via `emit_op1(Op::SizeOf(ty_arg), …)` at `mir-lower/body/call/intrinsic.rs:1298`. Codegen answers it from `tc.repr(ty)` (`cranelift/inst.rs:626-629`, `llvm/inst.rs:728`). With no *value* of `Vec3` in any mono body, `Vec3` is never registered and `repr` falls to the scalar branch.

**Honest narrowing (the finder overstated the blast radius).** `PtrRead`/`PtrWrite`/`PtrNull`/`PtrTo`/`PtrCast`/`PtrBitcast`/`PtrFromAddress`/`StackAlloc` all produce or consume a value typed `T` or `Pointer[T]`, which the value walk (`mono/mod.rs:1141-1144`) plus the `Pointer` arm (`:1199`) does collect. Only `SizeOf`/`AlignOf` (result type `Int64`) genuinely escape. The second half of the gap — `collect_named_type_from_ty` (`:1175-1211`) recursing into `type_args`, `Pointer` pointees and tuple elements but never into struct fields — is largely self-healing, because constructing a struct puts field values in the body. Every existing `Layout.of[X]()` testdata site happens to pair it with `.cast[X]()`, which is why this has not shipped as a bug report.

**Fix.** (1) Factor the exhaustive `TyId`-carrying op list out of `substitute_op_type` into one `op_type(&Op) -> Option<TyId>` helper, and add an `Op1/Op2/Op3` arm to `collect_named_types` that feeds it through `collect_named_type_from_ty`. (2) Close the fixed point over struct fields / enum payloads. (3) Move the guard to where it can fire: report every `concrete_types` key that never reached `all_resolved`, and make `classify_named`'s and `struct_field_offset`'s misses a hard `CodegenError` rather than `Scalar(ptr)` / `0`.

---

#### A3. The thunk pass identifies the closure environment parameter by the magic name `"env"`

**Category** fragility / **Severity** medium / **Locations**
`lib/kestrel-mir/src/passes/thunk.rs:30`, `:60`, `:63`, `:69`, `:112`, `:185`, `:241`
`lib/kestrel-mir-lower/src/body/closure.rs:278`
`lib/kestrel-mir-lower/src/body/expr.rs:880`
`lib/kestrel-mir-lower/src/items/function_sig.rs:120`

**Evidence.** MIR lowering *synthesizes* the environment parameter and names it with a string literal: `func_def.params.push(ParamDef::new("env", env_val, env_ty, ParamConvention::Consuming))` at `closure.rs:277-282`. `run_thunk_pass`, in a different crate, re-derives that structural fact from the string on `FunctionDef`s it did not create:

```rust
let needs_env = target_func.params.first()
    .is_some_and(|p| p.name == "env" || p.name == "_env");            // thunk.rs:60-63
let target_params: Vec<_> = target_func.params.iter()
    .filter(|p| p.name != "self" && p.name != "env" && p.name != "_env")
    .cloned().collect();                                              // thunk.rs:66-71
```

and forwards the environment pointer in that slot (`thunk.rs:112`) or destroys it (`:185`).

Three things make this reachable rather than theoretical. (a) The pass's input set is unfiltered: `thunk.rs:23-40` scans every function for `ApplyPartial { callee: Callee::Direct { func: target, .. } }` with no `FunctionKind` check. (b) `lower_def` wraps a bare user function used at a `FuncThick` slot in a capture-less `ApplyPartial` (`mir-lower/body/expr.rs:871-881`), so plain user functions enter that set. (c) `ParamDef.name` really is the user's source identifier — `function_sig.rs:120` builds it from `&ast_param.name`, unlike the synthetic `"self"` at `function_sig.rs:101` — and `env` is not a reserved word (the lexer declares 94 `#[token(...)]`s, none of which is `env`).

**Why it matters.** "Which parameter is the closure environment?" is a fact MIR-lower knows for certain and then throws away, forcing a downstream crate to recover it from a string. No `FunctionKind` or `ParamDef` flag carries it, and nothing validates the agreement. `verify_ossa` has no `InstKind::Call` arity or type check (it checks block-argument arity only), and it runs at a later stage than `Stage::Thunk` regardless.

**Failure scenario.**

```kestrel
func twice(env: Int64) -> Int64 { env * 2 }
func apply(f: (Int64) -> Int64, v: Int64) -> Int64 { f(v) }
// apply(f: twice, v: 21) — expected 42
```

`needs_env` is true, `target_params` filters the parameter out, and the generated `twice.thunk` declares one parameter (`_env: Pointer[()]`) while forwarding `_env` as `twice`'s `Int64` argument. Renaming the parameter to anything else makes the program correct — the miscompile is triggered purely by an identifier choice. The `p.name != "self"` twin on the same line is the identical shape and is **also live**. `self` is not reserved — it is neither a lexer token nor a parser keyword, and `func combine(self: Int64, x: Int64) -> Int64 { self * 100 + x }` compiles today — so the twin strips a real parameter from any free function that declares one, producing an arity mismatch the backend reports as an unexplained verifier failure. (Genuine *method* values are a separate matter and never reach this pass by the `instance.method` route: that lowers to `HirExpr::Field` and is rejected with `error[E100] method '…' must be called`. `Type.instanceMethod` **does** reach it, as an unbound reference the front end should reject but does not — see `docs/fragility/G3/decisions.md` §2b.)

**Fix.** Carry the fact. Add `ParamDef { is_env: bool }` set at `closure.rs:277`, or check a `FunctionKind::Closure` discriminant, and have `thunk.rs:60-71` test that instead of the string. The pass already reads `FunctionKind` elsewhere. At minimum, assert that a `needs_env` target is a closure function.

---

#### A4. `--target` reaches only `@platform` filtering; layout and both backends hardcode the host

**Category** single-source-of-truth / **Severity** medium / **Locations**
`src/main.rs:64`, `:259`, `:272`, `:338`, `:525`, `:561`, `:573`
`lib/kestrel-compiler/src/lib.rs:169`, `:223`, `:311`, `:330`, `:339`
`docs/tooling.md:39`
`lang/std/os/platform.ks:22`, `lang/std/io/libc.ks:60`

**Evidence.** `--target` is declared at `src/main.rs:64` under the doc comment "Target triple for cross-compilation". A repo-wide grep for `with_target|codegen_target|ast_target` over `lib/` and `src/` returns exactly **two** consumers: `src/main.rs:525` (`Compiler::new().with_target(self.ast_target())`) and `src/main.rs:338` (the `dump cranelift` arm). `ast_target()` (`src/main.rs:573-586`) substring-matches the triple and produces only an `Os`. `fn build` (`src/main.rs:206`) never calls `codegen_target()`; it calls `compile_and_link` (`:259`) / `compile_and_link_llvm` (`:272`), which hardcode `kestrel_codegen::TargetConfig::host()` at `lib/kestrel-compiler/src/lib.rs:311` and `:330`, and `kestrel_mir::TargetConfig::host_64()` at `:223`, `:265`, `:339`, `:413`. Linking is host-only too: `lib/kestrel-codegen-cranelift/src/link.rs:18` and `lib/kestrel-codegen-llvm/src/link.rs:22` are both `std::env::var("CC").unwrap_or_else(|_| "cc".to_string())`.

Meanwhile the flag *fully* controls which stdlib declarations exist: `Compiler::build` passes `self.target` into `build_declarations` (`lib/kestrel-compiler/src/lib.rs:169`), and the whole stdlib is routed through it.

**Why it matters.** The flag's documented contract and its actual reach disagree, and there is no place where the ast-builder `Os` and the codegen `Triple` are reconciled — so nothing can detect that they describe different machines. `Compiler::from_snapshot` also hardcodes `TargetConfig::host()` (`lib.rs:84`), so the test suite can never pin a target.

**Failure scenario.** On macOS: `kestrel build --target x86_64-unknown-linux-gnu hello.ks`. `ast_target()` yields `os: Some(Linux)`, so the darwin `platform()` (`lang/std/os/platform.ks:22-23`) is dropped and the linux one kept; `O_CREAT()` becomes `0x0040` (`lang/std/io/libc.ks:64-65`) instead of darwin's `0x0200` (`:60-61`). Codegen runs with `TargetConfig::host()` and the native ISA, and links with the host `cc`. Result: a working aarch64 Mach-O binary that prints "linux" and passes Linux `open(2)` flag values to the macOS kernel. Exit code 0, zero diagnostics. (Programs that touch `errno` will instead hit an undefined-symbol link error on `__errno_location`, `lang/std/io/libc.ks:53-55` — loud, not silent. The genuinely silent case is a program using only `platform()` or the wrong `O_*` constants.)

**Fix.** Parse the triple once into a struct yielding both `Os` and `pointer_width`, store it on `Compiler`, and have `compile_and_link`/`compile_and_link_llvm`/`monomorphize_mir` read it instead of calling `host()`/`host_64()`. Until real cross-compilation works, reject `--target` on `build` when the parsed triple's OS/arch differs from the host.

---

#### A5. Two independent pointer widths and two independent size tables

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-mir/src/item/mod.rs:96`, `:101`
`lib/kestrel-mir/src/passes/layout.rs:111`, `:120`
`lib/kestrel-codegen/src/target.rs:38`, `:61`
`lib/kestrel-codegen-cranelift/src/context.rs:49`, `:55`, `src/ty.rs:120`, `:128`
`lib/kestrel-codegen-llvm/src/ty.rs:126`
`src/main.rs:334`, `:338`

**Evidence.** `kestrel_mir::TargetConfig` has one field, `pointer_width: u64` (`item/mod.rs:96-98`), and its only constructor is `host_64()`, which unconditionally returns 8 (`:101-103`). MIR layout uses it to size every pointer-shaped type: `MirTy::Str => Some((target.pointer_width * 2, target.pointer_width))` (`layout.rs:111`), `Pointer`/`Ref`/`FuncThin` (`:112-115`), `FuncThick => pointer_width * func_thick_words(kind)` (`:120-123`).

Both backends **re-derive the same rules from a different source**: cranelift's `TypeCache` computes `MirTy::Str => Aggregate { size: ptr_size * 2, align: ptr_size }` (`cranelift/ty.rs:120-123`) and the same `FuncThick` rule (`:128-131`) from `target.pointer_size()` (`codegen/target.rs:61`), which comes from the *triple* (`from_triple`, `:38-49`). LLVM does the same and documents the hazard: `// ScalarTy::Ptr::bytes() is hardcoded to 8, so a non-64-bit target would mis-size pointer fields` followed by `debug_assert_eq!(ptr_size, 8, ...)` (`llvm/ty.rs:124-129`) — an assert compiled out of release builds.

Separately, cranelift's ISA is `cranelift_native::builder()` — always the host (`context.rs:49`) — while `ptr_ty` six lines later comes from the requested triple (`context.rs:55-59`).

**Why it matters.** The *duplication* is sanctioned: `layout.rs:118` says "(lockstep 9: layout ↔ both codegen `ty.rs` ↔ both `compile_apply_partial`)" and `lib/kestrel-codegen-llvm/AGENTS.md:44` repeats it. What is **not** blessed anywhere is that the two sides read pointer width from different sources — one a hardcoded `8`, the other the triple. Named-struct offsets come from the MIR layout while tuple/Str/closure sizes are recomputed in codegen, so a divergence is not locally detectable.

**Failure scenario.** `kestrel dump cranelift --target i686-unknown-linux-gnu f.ks` — the one code path that honors the flag. `monomorphize_mir` (`src/main.rs:334`) lays a `String` field out as 16 bytes / align 8; `cranelift_backend::compile` receives `pointer_size() == 4` and reprs the same `Str` as 8/4, with `ptr_ty = I32`, while the ISA is the host's. The emitted CLIF has 4-byte pointers on a 64-bit ISA and field accesses that disagree with the MIR offsets, with no error.

**Honest scoping.** This is unreachable on the real build path, because `compile_and_link` passes `TargetConfig::host()`, making triple, ISA and pointer width mutually consistent. It is reachable only through `dump cranelift`, which emits no artifact. It is reported because it is the structural precondition for every cross-compilation attempt and because the `debug_assert` is the only guard.

**Fix.** Delete `kestrel_mir::TargetConfig::host_64()` and thread the codegen `TargetConfig`'s `pointer_size()` into `monomorphize`/`run_pipeline`. Have codegen ask the MIR layout for Str/FuncThick sizes instead of recomputing them. Build the cranelift ISA from `target.triple` via `isa::lookup`, or hard-error when the triple is not the host.

---

#### A6. `@platform` fails open on every argument it does not recognise, and the validator its comments defer to does not exist

**Category** fragility / **Severity** medium / **Locations**
`lib/kestrel-ast-builder/src/build.rs:182`, `:192`, `:200`, `:208`, `:223`, `:228`
`lib/kestrel-ast-builder/src/components.rs:358`
`lib/kestrel-analyze/src/compilation/unknown_attribute.rs:42`, `:73`
`src/main.rs:573`, `:583`
`lang/clutch/src/Os.ks:14`, `lang/std/io/libc.ks:49`

**Evidence.** `is_excluded_by_platform` (`build.rs:191`) returns `false` — "include this declaration" — on five distinct paths: no target OS (`:192-194`), no `AttributeArgs` (`:200`, comment "let validation report"), no `AttributeArg` (`:208`), `Os::from_name` returns `None` (`:221-224`, comment "unknown platform — let validation report"), no implicit-member token (`:228`). `Os::from_name` accepts exactly `"darwin"` and `"linux"` (`components.rs:358-364`).

The validator those two comments defer to does not exist. A case-insensitive grep for `platform` across every `.rs` in `lib/` and `src/` returns only `build.rs`, `components.rs`, `src/main.rs`, a doc comment in `kestrel-compiler/src/lib.rs`, and `unknown_attribute.rs` — whose `KNOWN_ATTRIBUTES` allowlists the *name* `"platform"` (`:42-49`) and whose only check is `if !KNOWN_ATTRIBUTES.contains(&attr.name.as_str())` (`:73`). The argument is never read. On the CLI side, `ast_target()` yields `os: None` for anything without "darwin"/"apple"/"linux" (`src/main.rs:578-584`), and `build` never calls `from_triple`, so an invalid triple is never rejected.

**Why it matters.** An attribute whose entire purpose is to *exclude* code defaults to *including* it whenever the compiler cannot understand it, across the 35 `@platform` sites in `lang/`. The convention is systemic, not accidental — `lib/kestrel-test-suite/src/annotation.rs:167-171` applies the same lenient policy to the `backends:` header and says so.

**Failure scenario.** (a) A mistyped or unsupported triple (`wasm32-wasi`, `x86-64-linux`) makes `ast_target` return `os: None`, and *both* halves of all 15 darwin/linux pairs in `lang/std` survive into one module — e.g. both `__errno_ptr` externs (`lang/std/io/libc.ks:49-55`, one binding `__error`, one `__errno_location`) and both `platform()` definitions. The user gets a wall of E426 duplicate-signature errors pointing at stdlib files, with nothing naming the bad triple. Loud, but the diagnostic is useless. (b) A typo in an *unpaired* declaration is entirely silent: `@platform(.darwn)` on `lang/clutch/src/Os.ks:14` (an `@extern` to `_NSGetArgc`, darwin-only, no linux twin) compiles the macOS-only symbol into a Linux build. So does `@platform(.windows)` — a spelling the language does not support.

**Fix.** Add a decl-level check that reads the argument and errors on missing/unparseable/unknown OS names, so the two "let validation report" comments become true. Make `is_excluded_by_platform` fail closed on an unrecognised name. Validate `--target` with `from_triple` on the `build` path and reject a triple `ast_target` cannot classify, instead of degrading to `os: None`.

---

#### A7. "Does this loop diverge?" is answered at six sites; `guard.rs` alone omits the break check, punching a hole in the E003 soundness gate

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-analyze/src/body/guard.rs:147`, `:166`, `:167`
`lib/kestrel-analyze/src/body/dead_code.rs:198`
`lib/kestrel-analyze/src/body/exhaustive_return.rs:248`
`lib/kestrel-analyze/src/body/definite_assignment.rs:327`
`lib/kestrel-analyze/src/body/move_tracking.rs:634`
`lib/kestrel-hir-lower/src/desugar.rs:366`
`lib/kestrel-type-infer/src/generate.rs:603`
`lib/kestrel-hir-lower/src/stmt.rs:210`, `:226`
`docs/error-codes.md:37`

**Evidence.** Five analyzers gate loop divergence on a break check: `dead_code.rs:198` (`if block_contains_break(hir, body) { return false; }`), `exhaustive_return.rs:248`, `definite_assignment.rs:327`, `move_tracking.rs:634`. `guard.rs:167` is `HirExpr::Loop { .. } => true,` with **no** break check — and its own comment at `:166` reads "Infinite loop (no break) diverges", so the omission is an oversight, not a decision. A grep for `HirExpr::Loop { .. } => true` across `lib/` returns `guard.rs:167` and nothing else.

The Never-type shortcut at `guard.rs:147` does not save it: `desugar_while` (`lib/kestrel-hir-lower/src/desugar.rs:322-374`) emits a bare `HirExpr::Loop` whose synthetic unlabeled `Break` reaches `lib/kestrel-type-infer/src/generate.rs:603`, which unifies the loop's `break_tv` with `ctx.tuple(vec![])`. A `while` loop is therefore typed `()`, not `Never`, and control reaches `:167`.

**Why it matters.** E003 is Error severity and is the **only** gate enforcing `lower_guard`'s contract. `lib/kestrel-hir-lower/src/stmt.rs:210` desugars `guard c else { B }` to a bare `if c { } else { B }` *statement* pushed onto `guard_stmts` (`:226`); the rest of the function follows that `if`. If `B` does not diverge, execution simply continues past the guard with the condition FALSE. `docs/error-codes.md:37` states the invariant: "execution cannot fall through it." No other analyzer re-checks it — E003 is registered to `guard.rs` alone.

**Failure scenario.**

```kestrel
func clamp(x: Int64) -> Int64 {
    var i = 0;
    guard x > 0 else {
        while i < 3 { i = i + 1; }
    }
    return x;
}
```

E003 is not emitted. At runtime the `while` terminates, control falls out of the else block, and `return x` executes with `x <= 0` — exactly the state the guard excludes. The identical program checked by `dead_code.rs`'s or `exhaustive_return.rs`'s copy of the rule is correctly reported. `lib/kestrel-test-suite/testdata/patterns/guard_let/divergence/` has six files covering `break`, `continue`, `return`, `fatalError` and `-> !` — `grep -n loop` over them returns nothing.

**Fix.** Hoist one `loop_diverges(cx, block)` into `kestrel-analyze` and have `guard.rs`, `dead_code.rs`, `exhaustive_return.rs`, `definite_assignment.rs`, `initializer.rs` and `move_tracking.rs` all call it. `lib/kestrel-analyze/AGENTS.md` §5 sanctions per-analyzer *private* divergence helpers; it does not sanction one of them giving a different answer, and the same file's "One analyzer per fact" section warns about exactly this drift.

---

#### A8. All four copies of `expr_contains_break` ignore `Break.label`, disagreeing with MIR's `find_loop` — and it produces a false E002 on a file already in the test suite

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-analyze/src/body/dead_code.rs:189`, `:242`, `:318`, `:320`, `:334`
`lib/kestrel-analyze/src/body/exhaustive_return.rs:324`, `:326`, `:343`
`lib/kestrel-analyze/src/body/definite_assignment.rs:516`, `:518`, `:532`
`lib/kestrel-analyze/src/body/move_tracking.rs:2181`, `:2194`
`lib/kestrel-mir-lower/src/body/control.rs:429`
`lib/kestrel-mir-lower/src/body/mod.rs:717`
`lib/kestrel-test-suite/testdata/memory_model/deinit/break_from_nested_loop_3_levels.ks:22`

**Evidence.** `block_contains_break`/`stmt_contains_break`/`expr_contains_break` are triplicated **byte-for-byte** in `dead_code.rs`, `exhaustive_return.rs` and `definite_assignment.rs` (a `diff` of `dead_code.rs:296-337` against `definite_assignment.rs:494-535` reports only comment lines), plus a **fourth** copy in `move_tracking.rs:2159-2194` that the main report and the finder both missed. All four match `HirExpr::Break { .. } => true` — the `label` field discarded — and all four return `false` for `HirExpr::Loop` on the stated premise "their breaks target the inner loop", which is exactly what a labeled break violates.

MIR does it correctly: `find_loop` searches `loop_stack.iter().rev()` for `l.label.as_deref() == Some(label)` (`mir-lower/body/control.rs:429-437`). `dead_code.rs` already knows labels exist — its *other* divergence function is label-aware (`:189-191`: `Break { label, .. } | Continue { label, .. } => in_loop && label.is_none()`) — and `:242` carries an `#[allow(dead_code)] fn body_has_loop_label`, a vestige of an abandoned label-aware pass.

**Why it matters.** Two independent wrong answers from one omission: a `break outer` inside a nested loop is invisible to the outer loop (judged infinite), and makes the inner loop look breakable when it never falls through. This is the analyze-vs-MIR mismatch: the analyzer's model of which loop a break exits and MIR's are different functions with different rules.

**Failure scenario — already checked in and live.** `lib/kestrel-test-suite/testdata/memory_model/deinit/break_from_nested_loop_3_levels.ks:22-29` is `outer: loop { let mid_r = …; loop { let inner_r = …; break outer; } }` with reachable code on line 32. Trace `dead_code.rs`: `block_contains_break(outer body)` → the inner-Loop statement → `expr_contains_break(Loop)` → **false** (`:334`). So `expr_diverges` (`:206`) returns true, `check_block` sets `diverged`, and `:73` emits a **false E002 "unreachable code"** on a line that runs.

The E001 half kills a hard error: `func f() -> Int64 { outer: loop { loop { break outer; } } }` — `exhaustive_return.rs:248` sees no break in the outer body, `:255` returns `ReturnState::Diverges`, `definitely_returns()` is true, so **E001 is suppressed**. MIR then emits `TerminatorKind::Return(<unit literal>)` for a function whose `FunctionDef::ret` is `i64` (`mir-lower/body/mod.rs:717`). This half needs a bare outer `loop` — a desugared `while` carries its implicit `if cond {} else { break }` as the first body statement, so `block_contains_break` finds it and E001 fires correctly for while-shaped code.

**Fix.** `contains_break_for(hir, block, loop_label)` returning true only for `Break{label: None}` or a matching label, and recursing into nested `HirExpr::Loop` bodies whose own label differs. Mirror `control.rs:429` exactly. Since the four copies are byte-identical today, hoist the fixed version into `util.rs`; then either use or delete `body_has_loop_label`.

---

#### A9. `initializer.rs` runs a private break-state stack that pairs every `break` with the innermost loop, disabling the entire "all fields initialized" check

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-analyze/src/body/initializer.rs:195`, `:586`, `:587`, `:602`, `:603`, `:605`, `:616`, `:621`, `:622`
`lib/kestrel-mir-lower/src/body/control.rs:429`

**Evidence.** Unlike its five siblings, `initializer.rs` models loops with an explicit stack. `HirExpr::Loop` pushes a frame (`:586-587`), and after the body `:602-605` reads it: `if break_states.is_empty() { state.diverged = true; }` under the comment "No reachable break → infinite loop". The `Break` arm at `:621-625` is:

```rust
HirExpr::Break { .. } => {
    if let Some(top) = vctx.loop_break_stack.last_mut() { top.push(state.clone()); }
}
```

— `label` destructured away, state always pushed onto the **innermost** frame. MIR does the opposite (`control.rs:429-437`). The consumer is a single flag: `if !final_state.diverged { …report uninitialized fields… }` at `:195`.

**Why it matters.** This is a sixth independent control-flow model in one crate, and its break→loop pairing is the one fact MIR is authoritative about. When it mis-pairs, the outer frame ends up empty, `:605` declares an infinite loop, and the field-initialization check at `:195` is skipped **entirely** — not weakened, skipped. E005 is Error severity and is emitted only from this file. The escape is silent.

**Failure scenario.**

```kestrel
struct S {
    var a: Int64
    init() {
        outer: loop {
            loop { break outer; }
        }
    }
}
```

Outer pushes frame A; inner pushes frame B; `break outer` pushes onto `last_mut()` = **frame B**. The inner loop pops B (non-empty) and returns early at `:616`; the outer loop pops **frame A, empty** → `:605` sets `diverged` → `:195` never reports "initializer does not initialize all fields: 'a'". MIR's `find_loop` sends the break to the *outer* exit block, so control really does leave with `a` never stored.

**Fix.** Key `loop_break_stack` frames by label and have `:621` select the frame the way `control.rs:429` does. Better: make break→loop resolution a shared helper used by both `kestrel-analyze` and `kestrel-mir-lower`. Also drop the belt-and-braces `state.diverged` gate at `:195` in favour of per-field path state, so a divergence misjudgement degrades one field rather than the whole check.

---

#### A10. The Never-typed divergence rule is copy-pasted into five analyzers; only `move_tracking` carries the load-bearing `Loop` carve-out, and `dead_code` reads no types at all

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-analyze/src/body/move_tracking.rs:964`, `:965`
`lib/kestrel-analyze/src/body/definite_assignment.rs:115`, `:315`, `:428`
`lib/kestrel-analyze/src/body/initializer.rs:616`, `:703`
`lib/kestrel-analyze/src/body/guard.rs:147`
`lib/kestrel-analyze/src/body/exhaustive_return.rs:206`, `:271`
`lib/kestrel-analyze/src/body/dead_code.rs:181`
`lib/kestrel-type-infer/src/generate.rs:603`, `src/solver.rs:198`

**Evidence.** Five analyzers end their expression walk with the same rule — `guard.rs:147`, `exhaustive_return.rs:271`, `definite_assignment.rs:428`, `initializer.rs:703`, `move_tracking.rs:964` — three of them under the verbatim comment "Unified divergence detection: any expression with Never type diverges". Only `move_tracking.rs:965` adds `&& !matches!(&hir.exprs[id], HirExpr::Loop { .. })`, with the rationale that "a Loop expression with a reachable `break` has its type inferred to Never in some cases even though post-loop code is reachable." `initializer.rs` sidesteps it a third way, via the early `return state;` at `:616`. `definite_assignment.rs` has neither guard, and its Loop arm at `:315-331` does not return early, so `:428` overrides it.

A sixth analyzer, `dead_code.rs`, takes no `TypedBody` at all: `fn expr_diverges(hir: &HirBody, id: HirExprId, in_loop: bool)` (`:181`), and the file's `use` block imports nothing from `kestrel-type-infer`.

The exception is load-bearing because such a loop genuinely exists: `break outer` unifies the *outer* loop's `break_tv` (`generate.rs:603`), leaving the inner loop's unresolved for `default_never_fallback` to pin to `Never` (`lib/kestrel-type-infer/src/solver.rs:198-206`).

Also: `exhaustive_return.rs:206-207` documents the shared signal as unsound ("Type inference gives every `loop` type `Never` regardless of whether it contains a reachable `break`") — and that comment is itself **stale**, since `generate.rs:603` unifies with unit when a break reaches the loop.

**Why it matters.** Once `state.diverged` is set, `definite_assignment.rs:114-117` (`if state.diverged { break; }`) stops walking the block, so E004 (Error) is silently skipped for everything downstream. Two analyzers, one HIR node, opposite reachability verdicts. Separately, `dead_code.rs`'s type-blindness means a call to a `-> !` function — a shape the project explicitly supports (`lib/kestrel-test-suite/testdata/patterns/guard_let/divergence/guard_else_never_returning_fn_diverges.ks:11`, and BUG-68 at `docs/bughunt.md:1204`) — is diverging to five analyzers and non-diverging to the sixth.

**Failure scenario.** The nested-labeled-break shape from A8, in a function with `var x: Int64; … x = 1; return x;` after the loop: `move_tracking` keeps analyzing, `definite_assignment` stops, and `x = 1` is never examined. Note this is over-determined with A8 — the same shape already sets `diverged` via the label-blind gate at `definite_assignment.rs:327` — so fixing A8 does not fix this, and vice versa.

**Fix.** One `util::expr_diverges(cx, id)` owning both the `ResolvedTy::Never` test and the `HirExpr::Loop` exception, with the Loop case delegating to the single label-aware `contains_break` from A8. Have `dead_code.rs` call it too, giving E002 the `-> !` coverage the other five have. Correct the stale comments at `exhaustive_return.rs:206` and `initializer.rs:584`.

---

#### A11. `dead_code.rs` never handles `HirExpr::Sugar`, so E002 is structurally blind inside every `for`-loop body in the language

**Category** fragility / **Severity** low / **Locations**
`lib/kestrel-analyze/src/body/dead_code.rs:30`, `:129`, `:141`, `:168`
`lib/kestrel-hir-lower/src/desugar.rs:711`
`lib/kestrel-analyze/src/body/definite_assignment.rs:421`
`lib/kestrel-analyze/src/body/move_tracking.rs:953`
`lib/kestrel-analyze/src/body/initializer.rs:696`

**Evidence.** `desugar_for_loop` returns `HirExpr::Sugar { kind: SugarKind::ForLoop, inner: … }` (`desugar.rs:711`) — every `for` body in Kestrel lives under a `Sugar` node. `dead_code.rs:141-167` matches only `If`, `Loop`, `Match`, `Block`, `Closure`, ending with `_ => {}` at `:168`; `Sugar` falls into `_`. `check_stmt_inner:129` reaches a for-loop statement as `HirStmt::Expr { expr }` and hands the Sugar id straight to `check_expr_inner`, which drops it.

Every other body analyzer handles Sugar transparently and says so in an identical comment — `definite_assignment.rs:421`, `move_tracking.rs:953`, `initializer.rs:696` — so this is an oversight against a crate convention. `dead_code.rs`'s divergence walkers (`:181`, `:273`, `:318`) have the same gap, as does `guard.rs:169`.

**Why it matters.** E002 is the only unreachable-code diagnostic, and this makes it inapplicable to the body of the most common loop form. The blind spot is invisible in review because `check_expr_inner` *looks* complete — `Loop` is handled; it is just never reached for `for`. No test coverage exists: `grep -rl E002 lib/kestrel-test-suite/testdata/` returns nothing.

**Failure scenario.** `for x in xs { return; print(x); }` produces no E002; the identical program written with `while` or `loop` does. The diagnostic depends on which loop keyword the user typed. The same hole hides dead code inside `try` and string-interpolation subtrees.

**Severity is low, not medium.** E002 is `Severity::Warning` (`dead_code.rs:30`) and the gap is one-directional: `Sugar` falling into `_ => false` in the divergence walkers is *conservative*, so no false diagnostic is ever produced. The cost is lost warnings only.

**Fix.** Add the transparent `Sugar` arm to `dead_code.rs:141`, `:181`, `:273`, `:318` and `guard.rs:169`. To prevent recurrence, replace the trailing `_ => {}` with an exhaustive match over `HirExpr` so a newly-added wrapper variant is a compile error — the discipline `decl/visibility.rs:271-317` already uses.

**Related, found while verifying.** `exhaustive_return.rs:187` has the same defect class with the *opposite* polarity: its tail classification `match &hir.exprs[tail] { If | Match | Loop | Block => state, _ => ReturnState::Returns }` omits `Sugar`, so a `for` loop in tail position scores as a value-producing leaf and would suppress E001. Masked today because such a body is already an inference error and `:77` bails on `!cx.typed.errors.is_empty()`.

---

#### A12. The extension-bound evaluator *skips* `Copyable`/`Cloneable` clauses because `type_satisfies` cannot answer them; conformance-completeness calls `type_satisfies` on exactly those clauses and gets a hard `false`

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-type-infer/src/conformance.rs:164`, `:295`, `:298`, `:391`
`lib/kestrel-analyze/src/compilation/conformance_completeness.rs:1652`
`lib/kestrel-type-infer/src/resolve.rs:511`
`lib/kestrel-name-res/src/conformances.rs:284`
`lang/std/core/copy.ks:14`

**Evidence.** `extension_bounds_hold_impl` documents and enforces the rule at `conformance.rs:295-300`: *"Copyable / Cloneable are copy-semantics, not declared conformances; `type_satisfies` (which goes through `ConformingProtocols`) can't answer them. Skip"* — `if is_copy_builtin(ctx, *pb, root) { continue; }` at `:298`, helper at `:391`. The reason is real: `nominal_satisfies` returns `false` when `ConformingProtocols(entity)` lacks the protocol (`:164-168`), and `ConformingProtocols` walks only declared `Conformances` + inheritance. Nothing synthesizes `Copyable` — a grep for `Builtin::Copyable` across `lib/` returns 13 hits, none in `kestrel-name-res` or `kestrel-ast-builder`. `lang/std/core/copy.ks:14` is a bare `public protocol Copyable {}`, and `Int64` declares 20 protocols, none of which reaches it. The language answers the question a different way — through the copy-semantics classifier (`resolve.rs:511-517`).

The analyzer's twin of the same evaluator has no such guard. `extension_clauses_entailed` at `conformance_completeness.rs:1646-1653`: any `Bound` whose param maps to a concrete binding is answered by `return type_satisfies(cx.query, &resolved_ty_to_hir(binding), *protocol, cx.root);` — including when `*protocol` is `Copyable`/`Cloneable`. This is the only unguarded `type_satisfies` call site in the tree.

**Why it matters.** Two implementations of "does this concrete binding satisfy this extension's where-clause bound?" disagree by construction on the two most common builtin bounds in the language. The solver-side copy documents *why* the question is unanswerable and skips; the analyzer-side copy asks anyway and treats the unanswerable `false` as proof of violation — the one thing `type_satisfies`' module contract (`conformance.rs:15-25`, "rejects only on a provable concrete violation") says it must never be used for.

**Failure scenario.** Take the shipped #213 fixture `lib/kestrel-test-suite/testdata/declarations/extensions/constrained_protocol_ext_witness.ks` and change its bound from `Equatable` to `Copyable`:

```kestrel
protocol Container[T] { func item() -> T; func dup() -> T }
extend Container[T] where T: Copyable { public func dup() -> T { self.item() } }
struct BoxC: Container[Int64] { var v: Int64; func item() -> Int64 { self.v } }
```

`type_satisfies(Struct{Int64}, Copyable)` returns `false` at `conformance.rs:168`, `dup` is dropped from `ProvidedMembers`, and E454 "type 'BoxC' does not implement method 'dup'" fires on a legal program — while `BoxC(v: 4).dup()` type-checks and routes, because `extension_bounds_hold` hits the `is_copy_builtin` skip and permits.

**Latent, not live.** No shipped stdlib or testdata triggers it: the only `where _: Copyable` extensions in `lang/std` (`rcbox.ks:183`, `pointer.ks:351`, `optional.ks:543`, `result.ks:447`, `error.ks:25`) are type extensions or conformance-adding, never `TypeMemberSource::ProtocolExtension`.

**Fix.** Move the copy-builtin skip *into* `type_satisfies`: add a `Copyable`/`Cloneable` arm next to the existing `Static` arm at `conformance.rs:67-76`, answering via the copy-semantics classifier that `resolve.rs:511-524` already uses. Then delete the guard at `:298` — every caller, guarded or not, gets one answer.

---

#### A13. An unmappable where-clause subject is a PERMIT in the solver's evaluator and a REJECT in the analyzer's, so every associated-type-subject clause on a protocol extension is unentailable

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-type-infer/src/conformance.rs:303`, `:306`
`lib/kestrel-type-infer/src/entailment.rs:9`, `:100`
`lib/kestrel-analyze/src/compilation/conformance_completeness.rs:1582`, `:1656`, `:1708`
`lib/kestrel-type-infer/src/solver.rs:4994`
`lang/std/iter/iterator.ks:846`

**Evidence.** Solver side: if a clause's `param` is neither in the target-arg substitution nor the extension's target entity, `continue; // Unknown param — permit (conservative).` (`conformance.rs:306`). Analyzer side: `constraint_entailed_by` → `bound_entailed` falls off the end with `false` (`entailment.rs:100`). The entailment module's own header states the invariant it breaks: *"This is the lightweight static-analysis cousin of `solve_conforms` in `solver.rs`. Both must agree about what conformance means"* (`entailment.rs:8-9`).

For a protocol extension whose clause subject is an **associated type** rather than a protocol type param, the analyzer can never map it: `build_protocol_param_substitution` returns an empty map whenever the protocol has no `TypeParams` (`conformance_completeness.rs:1708`), so `substitute_clause` passes the clause through unchanged (`:1676-1680`, `None => *param`) and `constraint_entailed_by` at `:1659` receives a `Bound { param: <the protocol's Item TypeAlias entity> }` that appears in no context clause. The solver, given the same extension, computes an empty substitution and hits the `continue`.

This shape is shipped stdlib: `lang/std/iter/iterator.ks:846` `extend Iterator where Item: Equatable {`, plus `:866`, `:1032`, `:1071`.

**Why it matters.** `TypeMembers` deliberately returns every candidate and delegates entailment to the caller, naming `constraint_entailed_by` as *the* shared filter (`kestrel-name-res/src/type_members.rs:7-11`) — but the solver never uses it; it uses `extension_bounds_hold` via `extension_where_clauses_satisfied` (`solver.rs:4994-5000`). Two filters over the same candidate set, with opposite defaults for the same input class, each documenting its own tier ("conservative permit" vs "conservative reject") rather than a shared rule.

**Failure scenario.** A protocol `Base { type Item; func raw() -> Item; func doubled() -> Item }` with `extend Base where Item: Equatable { public func doubled() -> Item { self.raw() } }` and `struct Counter: Base { type Item = Int64; func raw() -> Int64 { 21 } }`. `extension_clauses_entailed` (`:1582`) sees an empty `proto_subs`, the clause survives as `Bound { param: Base.Item }`, `constraint_entailed_by` returns `false`, `doubled` is dropped, and E454 fires on a legal program — while `Counter().doubled()` type-checks and lowers.

**Same divergence, opposite polarity.** `conformance.rs:303`'s `Some(*param) == target_entity` branch is the twin: for `where Self: Q` the solver substitutes the receiver and evaluates `type_satisfies(recv, Q)`, while the analyzer always rejects. That shape *is* shipped in testdata (`declarations/extensions/constrained_protocol_extension_applies.ks:11`, `protocol_extension_calls_constraint_method.ks:11`, `more_constrained_extension_wins.ks:16`, and two more) — latent only because none of those members witnesses a requirement.

**Fix.** Delete the second evaluator: have `extension_clauses_entailed` call `extension_bounds_hold` with a reified receiver, passing the conformance context's clauses as an extra substitution source. If the reject default must stay for param-to-param bounds, at minimum teach `build_protocol_param_substitution` to bind the protocol's *associated* types (from the conformer's `type Item = …`), so an associated-type subject maps to a concrete binding and takes the `type_satisfies` branch.

---

#### A14. E101's condition-conformance test is a private `ConformingProtocols` lookup that only understands `ResolvedTy::Named`

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-analyze/src/body/condition_check.rs:119`, `:156`, `:161`, `:189`
`lib/kestrel-type-infer/src/generate.rs:471`
`lib/kestrel-type-infer/src/resolve.rs:585`
`lib/kestrel-type-infer/src/conformance.rs:129`
`docs/plans/boolean-guard.md:60`

**Evidence.** Inference deliberately emits no constraint for a condition: `generate.rs:471-475` — *"Condition type is not constrained during inference. Like lib1, validation of BooleanConditional conformance happens in a later pass."* That later pass is the sole gate, and its conformance predicate is a seventh, private implementation:

```rust
fn conforms_to_protocol(cx: &BodyContext<'_>, ty: &ResolvedTy, protocol: Entity) -> bool {
    let ResolvedTy::Named { entity, .. } = ty else { return false; };   // condition_check.rs:161
    cx.query.query(ConformingProtocols { entity: *entity, root: cx.root }).contains(&protocol)
}
```

Every other member of the family answers the abstract cases: `TypeResolver::conforms_to` has a `TyKind::Param` arm routing through `collect_param_protocol_bounds` (`resolve.rs:585-587`) and a `SelfType` arm (`:570-577`); `type_satisfies` permits `Param`/`SelfType`/`Opaque`/`AssocProjection` at `conformance.rs:125-129` with the explicit comment "Abstract positions MUST be permitted so generic bodies aren't spuriously rejected". `condition_check` returns `false` for all of them — and its own `describe_type` has dedicated arms for exactly those cases (`:189-192`: "type parameter", "Self", "associated type", "opaque type"), so the false diagnostic renders with a sensible name.

**Why it matters.** E101 is the only enforcement point for condition typing (inference is silent by design), so this private predicate is load-bearing, yet it is the one member of the family that cannot see type-param bounds, protocol `Self`, or opaque bounds. The rule it enforces is stated as "Bool or conform to `BooleanConditional`" (`docs/plans/boolean-guard.md:60`) with no abstract-position carve-out. Coverage is all concrete conformers: `lib/kestrel-test-suite/testdata/builtins/boolean_conditional/generic_boolean_conditional.ks` declares `extend Box[T]: BooleanConditional` and contains no `if`.

**Failure scenario.**

```kestrel
func pick[T](flag: T) -> lang.i64 where T: BooleanConditional {
    if flag { 1 } else { 0 }
}
```

`expr_types[flag]` is `ResolvedTy::Param`, `conforms_to_protocol` bails at `:161`, and E101 fires — "expected Bool, found type parameter" — on a program whose bound the compiler's own `TypeResolver::conforms_to(Param{T}, BooleanConditional)` answers `true`.

**One correction to the finder's second scenario.** `extend BooleanConditional { … if self … }` would *also* be answered `false` by `TypeResolver::conforms_to`, because `ConformingProtocols` collects a protocol's parents but never the protocol itself (`kestrel-name-res/src/conformances.rs:295-316`). Routing through the shared predicate would not repair that case; it needs the `SelfType` arm's semantics extended.

**Fix.** Build a `TyKind` from the `ResolvedTy` and call the same `TypeResolver::conforms_to` path the solver uses, or expose a `QueryContext`-level `conforms_to(ResolvedTy, protocol)` helper in `kestrel-type-infer` and have both call it. At minimum add `Param`/`SelfType`/`Opaque`/`AssocProjection` permit arms, matching `type_satisfies`' documented contract.

---

#### A15. `TypeResolver` re-derives where-clause bounds from raw `AstWhereClause`, collapsing `T.Assoc: P` onto the bare associated type

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-type-infer/src/resolve.rs:184`, `:589`, `:1610`, `:1612`, `:2224`, `:2240`
`lib/kestrel-type-infer/src/where_clauses.rs:5`, `:66`
`lang/std/iter/adapters.ks:866`

**Evidence.** The prohibition is written twice, immediately above the code that violates it. `resolve.rs:184-187`: *"Note: where clauses are not exposed on this trait. Callers use `crate::where_clauses::WhereClausesOf { entity, root }` directly — that query resolves names in the entity's own scope, avoiding the 'ambient owner' leak that motivated this design."* And `resolve.rs:1608-1611`, directly above `resolve_type_entity`: *"For type-definition-scoped resolution (e.g. where clauses), use the `WhereClausesOf` query instead of threading `self.body_owner` through to non-body contexts."* `resolve_type_entity`'s body passes `context: self.body_owner` at `:1619`.

`gather_bounds_from_where_clause` (`resolve.rs:2224-2252`) does exactly what both comments forbid: `ctx.get::<AstWhereClause>(entity)` raw at `:2231`, `self.resolve_type_entity(subject)` at `:2240`, `resolved_subj == param_entity` at `:2241`. `WhereClausesOf` avoids both hazards deliberately — `where_clauses.rs:63-68`: *"A projection subject (`T.Assoc: P`) must keep its base — `resolve_type_entity` would collapse `T.Assoc` to `Assoc`, losing the receiver"* — and emits `WhereClause::ProjectionBound { base, assoc, … }` at `:82-87`.

The collapsed result feeds `collect_assoc_type_direct_bounds_inner` (`resolve.rs:1996-2046`, calling the walker at `:2022` and `:2035` over the whole owner chain), whose output answers `conforms_to(TyKind::AssocProjection { assoc, .. }, protocol)` at `resolve.rs:589-592` — keyed on `assoc` alone, `base` discarded.

**Why it matters.** A single `X.Item: P` clause anywhere in the owner chain is attributed to the protocol's `Item` alias entity globally, granting `P` to **every** `_.Item` projection visible in that scope. That is a wrong-*accept* — the direction that produces an unsound witness rather than a spurious diagnostic — and it is the only such finding in this batch. The projection form is shipped syntax: `lang/std/iter/adapters.ks:866` `public struct IntersperseIterator[I]: Iterator where I: Iterator, I: not Copyable, I.Item: Copyable {`.

**Failure scenario.**

```kestrel
protocol Feed { type Item; mutating func next() -> Item }
func eq[T](a: T, b: T) -> Bool where T: Equatable { a == b }

struct Pair[A, B] where A: Feed, B: Feed, A.Item: Equatable {
    var a: A; var b: B;
    mutating func sameB() -> Bool { eq(self.b.next(), self.b.next()) }
}
```

`self.b.next()` types as `AssocProjection { base: B, assoc: Feed.Item }`. The `Conforms` constraint for `eq`'s bound is answered at `resolve.rs:589-592` → `collect_assoc_type_protocol_bounds(Feed.Item)` → finds `A.Item: Equatable`, collapsed to the same entity → `Equatable` returned. `type_satisfies` permits the abstract projection (`conformance.rs:129`), so `solve_conforms` succeeds. `B.Item` was never bounded, and mono must find an `Equatable` witness for whatever concrete type the caller supplies.

**Fix.** Delete the raw AST walk in `gather_bounds_from_where_clause` / `collect_param_direct_bounds_inner` / `collect_assoc_type_direct_bounds_inner` and build all three on `WhereClausesOf { entity: <each ancestor> }`, matching `WhereClause::Bound` for params and `WhereClause::ProjectionBound { base, assoc, .. }` for projections — comparing **both** `base` and `assoc` in the `TyKind::AssocProjection` arm at `resolve.rs:589`. That also removes the `body_owner`-scoped resolution the trait comment already forbids.

---

#### A16. `constraint_entailed_by`'s "param-declared bounds" tier is dead for every `TypeParameter` subject

**Category** fragility / **Severity** low / **Locations**
`lib/kestrel-type-infer/src/entailment.rs:16`, `:79`, `:94`
`lib/kestrel-type-infer/src/where_clauses.rs:57`, `:178`
`lib/kestrel-ast-builder/src/builders/type_param.rs:40`
`lib/kestrel-ast-builder/src/builders/helpers.rs:277`
`lib/kestrel-type-infer/src/resolve.rs:2194`

**Evidence.** `bound_entailed` step 2 is documented at `entailment.rs:16-18` as *"Param-declared bounds — `WhereClausesOf(constraint.param)` returns bounds attached to the param's enclosing decl. Mirrors `collect_param_protocol_bounds` in `solver.rs`"*, implemented at `:79-98`. But `WhereClausesOf` reads the `AstWhereClause` component off the entity it is *given* (`where_clauses.rs:57`), and that component is set by `set_where_clause` (`helpers.rs:277`), whose eight call sites are struct/enum/protocol/extension/function/subscript/type-alias builders — never a type parameter. `build_type_parameters` (`type_param.rs:40-46`) spawns the `TypeParameter` with `NodeKind`/`Name`/`FileId`/`DeclSpan`/`CstNode` and attaches `TypeParams` to the *parent*. The two implicit-bound injectors both require `ctx.get::<TypeParams>(entity)` (`where_clauses.rs:178`, `:237`), which a `TypeParameter` never has. The predicate it claims to mirror does the opposite: `collect_param_direct_bounds_inner` explicitly walks `parent_of(param)` (`resolve.rs:2194`) and the owner ancestor chain (`:2207-2218`).

**Correction to the finder's title.** "The whole branch is unreachable" is **wrong**. `param` is not always a `TypeParameter`: `where_clauses.rs:330`/`:343` resolves a `Self` subject to the enclosing type/protocol entity, so `extend Filterable where Self: Sortable` yields `Bound { param: <protocol entity> }`, and `WhereClausesOf(<protocol>)` is non-empty whenever that protocol carries its own where clause. The subject can also be an associated-type `TypeAlias`, which `type_alias.rs:83` does give an `AstWhereClause`.

**Why it matters, and why it is low.** One of three advertised tiers is inert for the most common subject kind, so `constraint_entailed_by` is weaker than its documentation and weaker than the solver predicate it must agree with. But no concrete wrong answer was demonstrated: `build_proto_param_subs` already normalizes extension params to struct params (`conformance_completeness.rs:1168-1176`) and `collect_context_where_clauses` normalizes decl params (`:1810-1821`), and where tier 2 would help, tier 1 usually already covers it. This is a doc/impl mismatch with a latent failure mode, not a live break.

**Fix.** Replace `WhereClausesOf { entity: param }` with a walk of `parent_of(param)` and further ancestors, mirroring `resolve.rs:2185-2218` — or add a `WhereClausesOnParam { param, root }` query that owns the parent walk once, so `entailment.rs` and `resolve.rs` share it. Add a unit test alongside `entailment.rs:163` that declares `struct Owner[T] where T: P` and asserts `T: P` is entailed from an empty context; today it fails.

---

#### A17. `KESTREL_COPYPROP_LIMIT` silently changes emitted code from inside a MIR pass and is documented nowhere

**Category** global-state / **Severity** low / **Locations**
`lib/kestrel-mir/src/passes/copy_propagation.rs:27`, `:34`, `:44`
`lib/kestrel-compiler/src/lib.rs:357`, `:431`
`docs/tooling.md:79`, `:85`

**Evidence.** `eliminate_redundant_copies` (`copy_propagation.rs:25`) reads `std::env::var("KESTREL_COPYPROP_LIMIT")` at `:27-30`, defaulting to `usize::MAX`, and uses it as a hard cap: `if total >= limit { break }` per function (`:34`) and `if total + func_total >= limit { break }` per block (`:44`). With `=0` the first test breaks immediately and the pass is fully disabled. It runs unconditionally on the real build path (`lib/kestrel-compiler/src/lib.rs:357`, inside `monomorphize_mir`, reached by both backends) and on the staged path (`:431`); neither is `#[cfg(test)]`.

`docs/tooling.md:79` says "Two environment variables override build flags" and the table at `:85-87` lists `KESTREL_BACKEND`, `KESTREL_OPT`, `KESTREL_STD` (three rows under a "two" heading — a separate small doc bug). A grep for `COPYPROP` across `docs/` and every `AGENTS.md` returns nothing. It has no CLI flag, is not echoed by `--verbose` (`src/main.rs:227-229` prints only the output path), is not part of `Compiler`'s state (`lib/kestrel-compiler/src/lib.rs:41-49`), and is not in any hECS query key — MIR is produced by direct calls, not memoized queries, so nothing records that the emitted module depended on it.

**Why it matters.** It is the only one of the 18 `KESTREL_*` variables in the tree that changes emitted instructions while remaining undocumented; the neighbours (`KESTREL_DEBUG_COPYPROP`, `KESTREL_DEBUG_TAKEALIAS`, `KESTREL_AUDIT_DUP`, `KESTREL_DEBUG_CLONE`, `KESTREL_DUMP_LLVM_IR`) are all print-only.

**Failure scenario.** A developer exports `KESTREL_COPYPROP_LIMIT=0` to bisect a copy-propagation bug, then runs an unrelated `kestrel build` or a triage suite in the same shell. Copy-propagation is disabled for every subsequent build in that session; retained `CopyValue`/`DestroyValue` pairs become real clone/drop calls in `expand_destroy_copy`, changing performance and, for any latent expand-stage bug, behaviour. Nothing in the output, the binary, or the diagnostics says the pass was capped.

**Fix.** Promote it to `--copyprop-limit` recorded alongside `opt_level` in `CodegenOptions` so it travels with the build configuration, or gate it behind `cfg!(debug_assertions)`. Minimally: list it in `docs/tooling.md` and print it under `--verbose` when set. (`CC`, read at `lib/kestrel-codegen-cranelift/src/link.rs:18` and `lib/kestrel-codegen-llvm/src/link.rs:22`, is output-affecting by the same argument and also absent from that table — conventional, but worth a row.)

---

## What the Audit Still Has Not Covered

After two passes, roughly 168 files have been cited. The following are genuine blind spots, grouped by what it would take to close them.

### Closable by reading (a focused session each)

- **The `needs_drop` / copy-class predicate family, arm by arm.** Five implementations exist and are verified: `lib/kestrel-mir/src/ty_query.rs:308`, `passes/drop_fix.rs:32`, `mono/expand.rs:379`, `mono/audit.rs:477`, `passes/clone_shim.rs:623` (plus `mono/mod.rs:1086` `concrete_copy`). Lockstep 1 names six. A1 found an *ordering* defect between two of them. **Nobody has diffed the arms.** A per-`MirTy`-variant table of what each of the six answers is a mechanical, high-value exercise; any cell that disagrees is either a leak or a double free.
- **The rest of the lockstep registry.** Nine numbered constraints (`docs/plans/closure-kinds/closure-kinds-plan.md:439-458`), 26 referencing files. Constraint 1 (A1) and constraint 9 (A5) have now been touched. Constraints 2, 3, 4, 6, 7 and 8 — front-end capture-move vs MIR no-Take, owning-expand arms vs legacy teardown gating, `FuncThick` kind vs mangling vs witness matching, E212 retirement vs ref-binding lowering, provenance stamping, escaping-Cloneable classification — have had **zero** attention across both passes.
- **`lib/kestrel-mir/src/passes/drop_shim.rs` (921 lines), `mono/mangle.rs` (522), `mono/audit.rs` (592), `display.rs` (1027).** Zero citations in 107 findings. `drop_shim.rs` in particular decides how a type is destroyed; `mangle.rs` decides symbol identity, and the main report's #36 already flagged a "lossy pair" key in the neighbouring `WitnessCache`.
- **Enum layout and discriminant encoding.** `passes/layout.rs` computes `EnumLayout`; both backends re-derive `discriminant_width` (`cranelift/inst.rs:2059`, `llvm/inst.rs:2103` default to I32) while `compile_switch` defaults the identical miss to I64 (`cranelift/terminator.rs:215`, `llvm/terminator.rs:271`). Four sites, two different silent answers for one missing map entry — noted here, not investigated.
- **`kestrel-type-infer`'s `unify.rs`, `compare.rs`, and the solver's constraint-queue ordering.** Still entirely uncited after two passes. This round reached `conformance.rs`, `entailment.rs`, `where_clauses.rs` and parts of `resolve.rs`, and found four defects in the six-way conformance-predicate family; the unification kernel itself is untouched.
- **`kestrel-analyze`'s `move_tracking.rs` (2,200+ lines).** The six control-flow models are now covered (A7–A11), but the place/projection model, exclusivity checking, and the `decl/` analyzers are not.

### Closable only by running

- **Every one of the 17 findings above is read-derived.** None was compiled or executed. A1 (the `String`-field init leak), A2 (`Layout.of[Vec3]().size`), A3 (`func twice(env: Int64)` used as a value) and A8 (the false E002, already in-tree at `break_from_nested_loop_3_levels.ks:32`) are each a five-to-ten-line `.ks` file. A8 is the cheapest and the most likely to be observable immediately, since the fixture already exists.
- **The incremental substrate.** The main report's #19–#23 (`TypedBody` fingerprint, `snapshot()` accumulator reset, revision pruning, unwind safety) describe mechanisms, not observed regressions. Confirming or refuting them needs memo-hit counts before and after a targeted edit — a running session, not a reading one.
- **Both backends' instruction selection and aggregate ABI.** Cranelift and LLVM were diffed only where a finding pointed at them (`classify_named`, atomic RMW width, Bool discriminant, the four `struct_field_offset`/`discriminant_width` fallbacks). Calling conventions for aggregates, by-value vs by-reference thresholds, and struct-return handling have never been compared systematically. This needs differential execution — the same `.ks` under `KESTREL_BACKEND=cranelift` and `=llvm` — not reading.
- **Linking.** Both `link.rs` files shell out to `CC` or `cc` with the host defaults; the only triple use is `!matches!(target.triple.operating_system, OperatingSystem::Darwin)` at `cranelift/link.rs:74`, fed the host triple. Whether the produced objects are actually well-formed for any non-host target is unknown and cannot be read out of the source.

### Needs a design conversation with the maintainer

- **Which of the seven conformance predicates is canonical.** `type_satisfies`, `nominal_satisfies`, `TypeResolver::conforms_to`, `extension_bounds_hold_impl`, `constraint_entailed_by`, `condition_check::conforms_to_protocol`, and the `ConformingProtocols` walk each answer a slightly different question with a self-declared precision tier. A12–A16 are four instances of the same disagreement. Collapsing them is not mechanical: the tiers exist because "abstract positions permit" and "concrete violations reject" are genuinely different contracts, and someone has to decide which callers need which.
- **Is `--target` meant to work at all?** A4/A5/A6 are three symptoms of one unresolved question. If cross-compilation is a goal, the three `TargetConfig` types need to become one and the ISA must come from the triple. If it is not, `--target` should be rejected on `build` and `docs/tooling.md:39` corrected. Either answer is cheap; leaving it ambiguous is what produces the silent host-binary-with-foreign-stdlib case.
- **Where does the lockstep registry live?** Nine cross-crate invariants indexed in a *completed plan document*, cited by number from 26 files. Whether that moves into `AGENTS.md`, into a doc-comment on a single owning type, or into an actual mechanism (a `#[test]` that asserts the six predicates agree on a fixture set of `MirTy`s) is a maintainer call. The mechanism version is the only one that survives the next contributor.
- **`kestrel-analyze`'s duplication policy.** `lib/kestrel-analyze/AGENTS.md` §5 explicitly sanctions per-analyzer private control-flow helpers, and that sanction is precisely what let six models drift into four different answers (A7–A11). The policy is defensible — analyzers should not couple — but it needs a carve-out for facts MIR is authoritative about ("which loop does this break exit", "does this loop diverge"). That is a rule change, not a patch.
- **Severity policy for missed diagnostics.** Five of the 17 findings are "an Error-level diagnostic is silently skipped" (A7 E003, A9 E005, A10 E004, A12/A13 false E454, A14 false E101). Whether a *missed* soundness gate is worse than a *false* one determines the fix order, and the two directions currently appear in the same audit at the same severity.