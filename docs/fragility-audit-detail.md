I spot-checked the citations for eight of the highest-ranked findings by reading the files directly (pattern-matching specialization, LSP rename, the OSSA verifier, closure `break` lowering, `StdlibCache`/`World::snapshot`, the stored-field predicates, `closure_box` init selection, and the E100 dedup sites). All held; two line numbers were off by one to three lines and are corrected below.

# Kestrel Compiler — Fragility & Single-Source-of-Truth Audit

## Summary

The compiler's defects cluster around one structural habit: **a semantic fact is decided in one place and then re-derived, by hand, in three to eight others**, with nothing — no shared function, no exhaustive match, no assertion — forcing them to agree. "Is this field a stored instance slot?", "which entity is the `Copyable` protocol?", "what does this `InferError` say?", "where is the stdlib?", "is this token trivia?" each have between three and eleven independent encodings. The second habit is **degraded answers that return silently**: `unwrap_or(BinaryOp::Add)`, `_ => SyntaxKind::Error`, `_ => ty.clone()`, `if let Some(info) = self.find_loop(label)` with an implicit no-op `else`, `find(...)?` on an intrinsic table. Neither habit fails loudly; both produce well-typed, well-formed output that means something other than what the source said.

The third theme is **hand-maintained parallel tables the compiler cannot check** — `SyntaxKind`'s 258 variants restated twice in the same file, `Builtin`'s four tables (one already has a hole), the intrinsic seeder vs. the intrinsic lowering table (105 names diverged). The fourth is **manual push/pop state without RAII** across three thread-locals and the query engine's own active-query stack, in a process where two hosts catch panics and keep the thread.

The good news: the memory-model core, the copy-fold kernel, and the hECS query engine are all deliberately designed with a single owner and documented invariants. Most of the debt is at the *edges* where crates re-ask a question the owner already answers — and most fixes are mechanical (delete a copy, call the owner). The genuinely risky ones are ranked at the top.

## Top Risks (ranked)

| Rank | Finding | Category | Severity | Fails silently? | Where |
|---|---|---|---|---|---|
| 1 | Range-overlap specialization misroutes match arms in codegen | single-source-of-truth | **high** | **yes — wrong runtime answer** | `kestrel-pattern-matching` → `kestrel-mir-lower` |
| 2 | LSP rename replaces the whole `let` statement; params insert at byte 0 | fragility | **high** | **yes — destroys user code** | `kestrel-lsp/handlers/rename.rs` |
| 3 | "Stored instance field" re-derived 8 ways; MIR layout vs. memberwise init can misalign | single-source-of-truth | medium | **yes — field index shift** | mir-lower, type-infer, semantics, analyze, captures |
| 4 | Escaping-closure box `init` chosen by parameter count alone | fragility | medium | **yes** | `mir-lower/body/closure_box.rs` |
| 5 | `break`/`continue` inside a closure inside a loop lowers to a no-op | fragility | medium | **yes** | hir-lower ↔ mir-lower |
| 6 | `Copyable`/`Cloneable` identified by `name.ends_with()` in 5 MIR sites | single-source-of-truth | medium | **yes — body replaced by a trap** | `kestrel-mir/ty_query.rs`, `mono/mod.rs` |
| 7 | "Does T have a user clone?" answered by witness vs. by name; lookup last-write-wins | single-source-of-truth | medium | **yes — double free** | `kestrel-mir/passes/clone_shim.rs`, `mono/expand.rs` |
| 8 | LLVM backend never got the Bool-discriminant width fix (ef3fb801) | single-source-of-truth | medium | **yes — reads 3 bytes of stack** | `kestrel-codegen-llvm/inst.rs:249` |
| 9 | `NominalCopySemantics` memo depends on a thread-local stack not in its key | incremental-hazard | medium | **yes — order-dependent copy class** | `kestrel-semantics/lib.rs`, `staticness.rs` |
| 10 | Static-member lookup truncates to the first `extend` block | single-source-of-truth | medium | no (bogus error) | `kestrel-name-res/helpers.rs:118` |
| 11 | Solver and move-checker scope `TypeParamCopyRequirement` differently | single-source-of-truth | medium | no (spurious reject) | type-infer ↔ analyze ↔ semantics |
| 12 | Assoc types on type params matched by **name string**; E439 can't see extension params | fragility | medium | **yes — wrong assoc type** | `kestrel-name-res/resolve_type.rs` |
| 13 | Missing `;` after an expression statement is silently accepted | single-source-of-truth | medium | **yes** | `kestrel-parser/block/mod.rs:666` |
| 14 | Closure lowering's `SavedState` hand-mirrors `OssaBodyCtx`; 3 fields unsaved | side-table | medium | partly (panic or corruption) | `mir-lower/body/closure.rs:77` |
| 15 | `InferError` rendered twice by two divergent tables; LSP dedups neither | single-source-of-truth | medium | no (double squiggles) | compiler, analyze, main.rs, lsp |
| 16 | `TypedBody`'s hand-written `Hash` omits 5 of 11 output fields | incremental-hazard | medium | **yes — frozen diagnostics** | `type-infer/result.rs:91` |
| 17 | LSP despawns *before* `begin_revision()`, erasing all invalidation | ordering-dependency | medium | **yes — stale resolution** | `kestrel-lsp/compiler_worker.rs:311` |
| 18 | Recursion guards and the query stack are not unwind-safe | global-state | medium | **yes — cross-test poisoning** | hecs, semantics ×2, mir-lower |
| 19 | `snapshot()` clones memos but resets accumulators; both docs say the opposite | incremental-hazard | medium | **yes — diagnostics vanish** | `hecs/world.rs:335` |
| 20 | `add_token_or_missing` widens spans by one **byte** → LSP panics on Unicode | fragility | medium | **yes — whole publish dropped** | `parser/event.rs:189`, `lsp/position.rs:74` |

---

## Findings

### Silent miscompilation and wrong behavior

#### 1. Range-overlap correction lives only in `check_match`; decision-tree codegen uses the raw overlap test and misroutes arms

**Category** single-source-of-truth / **Severity** high / **Locations**
`lib/kestrel-pattern-matching/src/constructor.rs:140`, `:143`
`lib/kestrel-pattern-matching/src/flat_pat.rs:85`
`lib/kestrel-pattern-matching/src/matrix.rs:165`, `:232`
`lib/kestrel-pattern-matching/src/usefulness.rs:132`
`lib/kestrel-pattern-matching/src/decision_tree.rs:179`, `:221`
`lib/kestrel-mir-lower/src/body/pattern.rs:648`
`lib/kestrel-analyze/src/body/exhaustiveness.rs:72`

**Evidence.** `Constructor::matches` — documented as "the SINGLE compatibility check — no duplicates elsewhere" (constructor.rs:118-121) — is an *overlap* test, not containment:

```rust
(Constructor::IntLiteral(v), Constructor::IntRange { start, end }) =>
    start.is_none_or(|s| *v >= s) && end.is_none_or(|e| *v <= e),
(Constructor::IntRange{start:s1,end:e1}, Constructor::IntRange{start:s2,end:e2}) =>
    ranges_overlap_i64(*s1, *e1, *s2, *e2),
```

`FlatPat::decompose` gates row retention on it (`if !ctor.matches(target_ctor) { return None; }`, flat_pat.rs:85), so `specialize` keeps rows that only *partially* cover the target. The correction exists in exactly one consumer — `usefulness::check_match` recomputes coverage out-of-band with `range_covered_by_union_i64` (usefulness.rs:132-146). `decision_tree::compile_matrix` has no counterpart: it calls `matrix.specialize(..., ctor)` directly (decision_tree.rs:180) and `compile_leaf` takes `rows.first()` (decision_tree.rs:221).

The crate's own `AGENTS.md` requires the fix be "semantically correct for all consumers (diagnostics AND decision-tree codegen)."

**Why it matters.** MIR turns `cases` into `SwitchArm`s in source order (pattern.rs:648-660) and both backends lower `Switch` as a first-match comparison chain. The only guard is E307, whose `default_severity` is `Severity::Warning` (exhaustiveness.rs:72-77). The program compiles and silently runs the wrong arm.

**Failure scenario.** The crate's own testdata `patterns/exhaustiveness/overlapping_ranges.ks` (`0..=10 => "first"`, `5..=15 => "second"`, `_ => "other"`): specializing on `IntRange{5,15}` retains the `0..=10` row, which is row-first, so that case compiles to `Success(arm 0)`. `x = 12` returns `"first"`. Same for literal-before-range: `match n { 5 => A, 0..=10 => B, _ => C }` with `n = 3` returns `A`. That testdata file is `// test: diagnostics` only, so no execution test pins the runtime answer.

**Fix.** Split the column's int/char constructors into disjoint intervals before building cases in `compile_matrix` (the standard IntRange-splitting step), or add an asymmetric `Constructor::covers(&self, target)` used by `decompose` with pre-split targets. The rule must live in one function used by both `check_match` and `compile_matrix`, replacing the ad-hoc `range_covered_by_union_i64` patch.

---

#### 2. LSP local rename replaces the whole `let` statement (and inserts at file offset 0 for parameters)

**Category** fragility / **Severity** high / **Locations**
`lib/kestrel-lsp/src/handlers/rename.rs:238`, `:250`, `:254`, `:389`
`lib/kestrel-lsp/src/references.rs:192`
`lib/kestrel-hir-lower/src/stmt.rs:126`
`lib/kestrel-hir-lower/src/lib.rs:79`
`lib/kestrel-ast-builder/src/lower.rs:202`
`lib/kestrel-span/src/lib.rs:27`
`lib/kestrel-lsp/src/handlers/hover.rs:270`

**Evidence.** `kestrel_hir::res::Local` has one `span` field, and two consumers read it with opposite meanings.

Producer — the span is the **whole statement**: `lower_variable_decl` takes `let span = self.span(node);` for the `VariableDeclaration` node (lower.rs:202), and `lower_let_stmt` forwards it verbatim: `let local = self.define_local(name, is_mut, span.clone());` (stmt.rs:126). Parameters are worse: `lower.define_local(&param.name, param.is_mut, Span::synthetic(0))` (lib.rs:79), where `Span::synthetic` is `{file_id, start: 0, end: 0}`.

Consumer A treats it as a container and says so: `// Locals carry the enclosing let/var stmt span.` then `local.span.start <= ident_start && ident_end <= local.span.end` (hover.rs:270-273).

Consumer B treats it as the identifier: `Some((local.name.clone(), local.span.clone()))` (rename.rs:238), pushed as `RefKind::Direct` (rename.rs:250-261) — and `clip_to_identifier` short-circuits: `if matches!(kind, RefKind::Direct) { return span.clone(); }` (references.rs:192). `push_decl_site` takes the file from `entity_file(world, *body)` (rename.rs:254), not `span.file_id`, so a 0..0 param span lands at byte 0 of the real file.

**Why it matters.** Rename is a destructive refactor the user trusts blindly, and there is **no** `Target::Local` test — `rename.rs`'s test module (411-515) constructs `Target::Entity` only, and `lib/kestrel-lsp/tests/integration.rs` has no rename coverage.

**Failure scenario.** `func f() { let count = compute(1, 2); use(count); }` — cursor on `count` in `use(count)`, rename to `total`. The WorkspaceEdit replaces the entire `let` statement, yielding `func f() { total; use(total); }`; the call to `compute` is gone. Separately, in `func g(width: Int64) { width + 1 }`, renaming `width` emits an extra edit at range 0:0-0:0, producing `wmodule Demo`.

**Fix.** Give `Local` a `name_span` field set from the `BindingPattern` identifier in `define_local`, keeping `span` as the declaring-statement span for hover's containment test. Until then, reject `Target::Local` with a synthetic span (`start == end`) and re-derive the identifier span from the CST the way `hover::local_at_binding` already does.

---

#### 3. "Is this a stored instance field?" is re-derived in eight places with three different predicates

**Category** single-source-of-truth / **Severity** medium / **Locations**

*Layout authority (`!Callable && !Static`)*
`lib/kestrel-mir-lower/src/items/struct_lower.rs:29`, `:32`
`lib/kestrel-mir-lower/src/ty.rs:761`

*Memberwise-init authority (`!Computed && !Static`)*
`lib/kestrel-type-infer/src/generate.rs:1338-1344`

*Copy-semantics authority (`!Computed` only — statics ARE folded in)*
`lib/kestrel-semantics/src/lib.rs:744`

*Analyzers (no `Static` filter at all)*
`lib/kestrel-analyze/src/compilation/struct_cycles.rs:174`
`lib/kestrel-analyze/src/decl/recursive_enum.rs:185`
`lib/kestrel-analyze/src/decl/protocol_field_conformance.rs:66`
`lib/kestrel-analyze/src/body/initializer.rs:151`

*Correct ones (for contrast)*
`lib/kestrel-analyze/src/decl/field_method_collision.rs:63`, `lib/kestrel-analyze/src/decl/field.rs:166`, `lib/kestrel-type-infer/src/captures.rs:173`

**Evidence.** The MIR layout table — the definition of every `FieldIdx` — is:

```rust
// struct_lower.rs:28-34
if ctx.world.get::<NodeKind>(child) != Some(&NodeKind::Field) { continue; }
if ctx.world.get::<Callable>(child).is_some() || ctx.world.get::<Static>(child).is_some() { continue; }
```

`generate.rs:1337` carries the comment "(mirrors the layout collection in struct_lower.rs)" and then uses a *different* marker:

```rust
qctx.get::<NodeKind>(c) == Some(&NodeKind::Field)
    && qctx.get::<Computed>(c).is_none()
    && qctx.get::<Static>(c).is_none()
```

`Computed` ≠ `Callable`: the builder sets `Computed` for **any** `PropertyAccessors` block (`builders/field.rs:66`) but `Callable` only for a getter-with-body, the shorthand `{ expr }` form, or a `ref` clause (field.rs:117/132/143). The bodyless requirement form `{ get set }` therefore yields `Computed` without `Callable`, and E622 explicitly does not fire on it (`decl/place_accessor.rs:156-160`).

The two lists must be index-aligned, because `emit_struct_construct` maps argument position straight to field index with no arity or name check:

```rust
// mir-lower/body/call/mod.rs:772-779
let fields: Vec<(FieldIdx, ValueId)> = args.iter().enumerate()
    .map(|(i, arg)| { let val = self.lower_expr(arg.value); (FieldIdx::new(i), val) })
```

Separately, `captures.rs:171-172`'s doc comment admits its own duplication: "Mirrors the discrimination in `kestrel-mir-lower` `lower_field_access`" — and `lower_field_access` re-derives the same three discriminators inline as independent booleans (`expr.rs:700`, `:701`, `:707`).

**Failure scenarios.**
(a) *Index shift.* `struct S { var a: Int; var b: Int { get set }; var c: Int }`. `b` carries `Computed` but no `Callable` and no `Static`. type-infer's field list is `[a, c]` so `S(a: 1, c: 3)` type-checks; mir-lower's `def.fields` is `[a, b, c]`, so arg1 writes into `b` and `c` is never initialized. Silent garbage read.
(b) *Wrong copy class.* `struct Registry { static var current: Handle; var count: Int }` where `Handle: not Copyable`. `collect_child_types` does not filter `Static`, so `NominalCopySemantics(Registry)` is `NotCopyable`, and `lower_copy_behavior` stamps `CopyBehavior::None` on a struct whose only MIR field is `count: Int`. Every `let b = r` becomes a move and the move checker reports use-after-move on a struct with no non-copyable storage.
(c) `struct Config { public static let shared: Config = makeDefault(); let n: Int64 }` gets a false E449 "struct cannot contain itself" from `struct_cycles.rs`.

**Fix.** One query, `StoredInstanceFields { entity, root } -> Arc<Vec<Entity>>`, in `kestrel-semantics` or `kestrel-name-res`. Decide the predicate deliberately (`!Static && !Computed`, with the ast-builder guaranteeing `Callable ⟹ Computed`) and route all eight sites through it. Independently, make `emit_struct_construct` bind by field *name* via `resolve_field_idx` so a length/order drift is a hard miss, not a silent shift.

---

#### 4. Escaping-closure box `init` is picked by arity alone; `RcBox` already has two 1-parameter inits

**Category** fragility / **Severity** medium / **Locations**
`lib/kestrel-mir-lower/src/body/closure_box.rs:133`, `:199`, `:215`, `:44`
`lang/std/memory/rcbox.ks:79`, `:93`
`lang/std/memory/sharedbox.ks:71`

**Evidence.**

```rust
// closure_box.rs:132-133
let init =
    self.find_box_member(entity, NodeKind::Initializer, |c, _| c.params.len() == 1)?;
```

The predicate takes the member name as `_`. `find_box_member` walks `children_of(parent)` in declaration order and returns the first match with no ambiguity check (closure_box.rs:199-215) and no visibility filter. `RcBox` — the `@builtin(.SharedBox)` binding — declares two one-parameter initializers:

```kestrel
// rcbox.ks:79
public init(consuming value: T) { ... allocates RcBoxStorage, refCount 1 ... }
// rcbox.ks:93
private init(inner inner: Pointer[RcBoxStorage[T]]) { self.ptr = inner; }
```

The module's own doc contradicts the code: "Every member is found BY REQUIREMENT NAME on the binding entity — never by a hard-coded `RcBox` reference (plan D9)" (closure_box.rs:44) — `init` is the one member *not* found by name. Every sibling selector in the same crate disambiguates by label (`literal.rs:435`, `:451`; `witness_lower.rs:786-802`).

**Failure scenario.** Move `private init(inner:)` above `public init(consuming value:)` in `rcbox.ks` — a purely cosmetic reorder — or add any other 1-argument `init` to `RcBox`. Every escaping closure that captures state is then constructed with an initializer that expects an already-allocated `Pointer[RcBoxStorage[T]]` and merely stores it: no heap allocation, no refcount, no diagnostic. The env word becomes garbage.

**Fix.** Match on the `SharedBox.init(consuming value: Target)` protocol requirement via the conformance witness table, or at minimum on the argument label `value`. Make `find_box_member` return an error when more than one child satisfies a predicate.

---

#### 5. `break`/`continue` validation leaks across the closure boundary; MIR then silently no-ops the `break`

**Category** fragility / **Severity** medium / **Locations**
`lib/kestrel-hir-lower/src/ctx.rs:57`, `:195`
`lib/kestrel-hir-lower/src/expr.rs:1336`, `:1411`, `:1686`
`lib/kestrel-mir-lower/src/body/closure.rs:342`
`lib/kestrel-mir-lower/src/body/control.rs:332`, `:351`, `:357`, `:380`
`lib/kestrel-type-infer/src/generate.rs:1535`

**Evidence.** `LowerCtx` carries the loop stack for the whole body — `pub loop_labels: Vec<Option<String>>` (ctx.rs:57), `in_loop()` is `!self.loop_labels.is_empty()` (ctx.rs:195). `lower_closure` saves/restores the *scope* stack (`push_scope` expr.rs:1336, `pop_scope` expr.rs:1411) but never the loop stack — grep confirms the only `push_loop`/`pop_loop` sites are real loops. So inside a closure written in a loop body, `validate_break_continue` (expr.rs:1686) sees `in_loop() == true` and emits nothing.

MIR does the opposite: `lower_closure_expr` hands the closure a fresh loop stack (`loop_stack: mem::take(&mut self.loop_stack)`, closure.rs:342). `lower_break` then falls off the guard:

```rust
// control.rs:331-351
pub fn lower_break(&mut self, label: Option<&str>) -> ValueId {
    if let Some(info) = self.find_loop(label) { ... self.emit_jump(exit, exit_vals); }
    self.emit_literal(Immediate::unit())
}
```

No `else`, no diagnostic, no ICE. `lower_continue` has the identical shape (control.rs:357, :380). A third instance: `gen_closure` saves and restores `ctx.return_ty` but not `ctx.loop_break_tys` (generate.rs:1535, 1562, 1596), so a `break` inside a closure unifies against the *enclosing* loop's break TyVar.

**Failure scenario.**
```kestrel
outer: for i in 0..10 {
    items.forEach { x in
        if x > 3 { break outer; }
    };
}
```
`validate_break_continue` passes (`in_loop()` true, `has_loop_label("outer")` true). `lower_break` in the closure's own MIR function finds an empty `loop_stack` and emits a unit literal. The loop runs all 10 iterations; the `break` is a silent no-op. Bare `break` behaves identically. `break` outside *any* loop still errors correctly, so the hole is invisible in the obvious test.

**Fix.** `let saved = std::mem::take(&mut self.loop_labels);` at the top of `lower_closure`, restored after `pop_scope()`; same for `ctx.loop_break_tys` in `gen_closure`. Then turn the silent fall-through in `control.rs:332`/`:357` into an explicit ICE, since it becomes unreachable.

---

#### 6. `Copyable`/`Cloneable` lang protocols are identified by `name.ends_with(...)` in five MIR sites

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-mir/src/ty_query.rs:358`, `:365`, `:372`
`lib/kestrel-mir/src/mono/mod.rs:114`, `:269`
`lib/kestrel-mir-lower/src/context.rs:41`
`lang/std/core/copy.ks:13`, `:31`
`lib/kestrel-name-res/src/resolve_builtin.rs:133`

**Evidence.** The frontend binds these via lang items (`@builtin(.Copyable)` / `@builtin(.Cloneable)`, copy.ks:13,31) resolved by `ResolveBuiltin`. Every other layer uses that path — `analyze/body/move_tracking.rs:121`, `semantics/lib.rs:279`, `type-infer/where_clauses.rs:182`, `type-infer/solver.rs:2435`, `hir-lower/ty.rs:1266`. MIR does not:

```rust
is_cloneable_protocol:  ...is_some_and(|p| p.name.ends_with("Cloneable"))     // ty_query.rs:358
is_copyable_protocol:   ...is_some_and(|p| p.name.ends_with("Copyable"))      // ty_query.rs:365
find_cloneable_protocol: .values().find(|p| p.name.ends_with("Cloneable"))    // ty_query.rs:372
```

and `mono/mod.rs:114` re-inlines the third one rather than calling it. `ProtocolDef.name` is the *module-qualified* dotted path (`register_name` → `qualified_name`, mir-lower/context.rs:41-45), so any protocol whose last segment ends in those letters matches — `TriviallyCopyable`, `DeepCloneable`, not just a literal `Copyable`. The `find(...)` variants take whichever protocol comes first in `module.protocols` insertion order, i.e. lowering order.

**Why it matters.** These predicates drive `copy_behavior`'s `TypeParam` arm (`Bitwise` on a Copyable bound, `Clone(p)` on a Cloneable bound, ty_query.rs:199-215) and `copy_is_mono_dependent` (ty_query.rs:265).

**Failure scenario.** A user writes `protocol BitCopyable {}` plus `extend File: BitCopyable {}` and `func f[T](x: T) where T: BitCopyable`. `violated_copyable_bound` (mono/mod.rs:266-269) classifies the bound as Copyable; for `T = File` (move-only), `poison_body` (mono/mod.rs:333-344) replaces `f`'s entire body with `TerminatorKind::Panic("... requires 'Copyable' (bound not satisfied)")`. A legal program compiles cleanly and traps at runtime with a message about a protocol the user never named. The dual case — a user `DeepCloneable` lowered before `std.Cloneable` — makes `find_cloneable_protocol` return the wrong entity, so every synthesized clone shim registers as a witness of the user's protocol.

**Fix.** Resolve both once during MIR lowering via `ResolveBuiltin { builtin: Builtin::Copyable / Cloneable }`, store the `Entity`s on `MirModule`, and reduce all five predicates to equality checks. Delete the inline duplicate at `mono/mod.rs:114`.

---

#### 7. "Does T have a user clone?" is answered by witnesses in `clone_shim` but by method name in `expand`; the lookup silently last-write-wins

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-mir/src/passes/clone_shim.rs:73`, `:101`, `:224`
`lib/kestrel-mir/src/mono/expand.rs:234`, `:258`, `:328`
`lib/kestrel-mir/src/mono/collect.rs:513`
`lib/kestrel-mir/src/item/function.rs:205`

**Evidence.** `synthesize_clone_shims` decides from the **witness table** (`has_user_clone` = entities with a `WitnessDef` whose `protocol == cloneable_proto`, clone_shim.rs:73-84, applied at :101/:111). `build_clone_lookup` in expand decides from the **name**: `FunctionKind::Method { parent, .. } if f.name.ends_with(".clone")` (expand.rs:258), via `clone_method_self_nominal`'s `!self.name.ends_with(".clone")` guard (function.rs:205).

The codebase itself documents the case where they disagree — clone_shim.rs:218-223: *"a `clone()` defined in an `extend` block (vs inline) doesn't always surface a witness here."* When they disagree, both `__clone$T` and `T.clone` land under the same `(nominal, type_args)` key, and the insert is unconditional:

```rust
// expand.rs:311-329
for (mi, mf) in module.functions.iter().enumerate() {
    if let Some(&nominal) = clone_func_to_parent.get(&mf.source) {
        lookup.insert((nominal, mf.type_args.clone()), MonoFuncId::new(mi));
```

No collision check; later mono index wins; `module.functions` order is instantiation-discovery order. There are in fact **four** predicates for "which function is T's clone": `build_clone_impl_to_nominal_map` (expand.rs:234) uses raw `*parent` with no fallback, and `discover_clone_shim` (collect.rs:513) prefers the shim.

**Why it matters.** `clone_lookup` is what `emit_clone_recursive` calls for every `CopyValue` on a Named type. Picking the memberwise shim over a hand-written `clone()` skips a retain — the exact double-free the surrounding comments say this code exists to prevent.

**Failure scenario.** A refcounted handle whose `clone()` is defined out-of-line and does not surface a `WitnessDef`: `__clone$RcHandle` is synthesized *and* registered as the Cloneable witness, `RcHandle.clone` is separately monomorphized, and if the shim is collected later it overwrites the user entry. `let b = a` becomes a memberwise copy of the box pointer with no `retain()`; both handles release → double free. Whether it breaks depends purely on instantiation order.

**Fix.** Add `FunctionKind::CloneImpl { nominal }` (or `clone_of: Option<Entity>` on `FunctionDef`) stamped once during MIR signature lowering where the Cloneable conformance is known, and have all four sites read it. Make `build_clone_lookup`'s insert a hard error on duplicate key.

---

#### 8. The LLVM backend never received the Bool-discriminant width fix that landed in cranelift (ef3fb801)

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-codegen-llvm/src/inst.rs:249`
`lib/kestrel-codegen-cranelift/src/inst.rs:249`
`lib/kestrel-codegen-llvm/src/ty.rs:2095`, `lib/kestrel-codegen-cranelift/src/ty.rs:2049`
`lib/kestrel-test-suite/testdata/expressions/match/regression/bool_match_with_wildcard_default.ks:1`

**Evidence.** Both backends default `discriminant_width` to I32 for a non-enum Named type. Cranelift's `@guaranteed` arm loads at the value's own width and normalizes (inst.rs:249-269), with the comment: *"loading `disc_width` bytes from a 1-byte value over-reads adjacent memory, so a `match b { true => .., _ => .. }` compared the tag against garbage and always took the default."* LLVM still has the pre-fix code:

```rust
// codegen-llvm/inst.rs:249-251
TypeRepr::Scalar(_) if is_guaranteed => builder
    .build_load(disc_ty, base.into_pointer_value(), "disc").unwrap(),
```

`git show --stat ef3fb801` confirms it touched only the cranelift file. Note the `@owned` arm of the LLVM version *does* normalize, so the omission is specifically the `@guaranteed` path. `kestrel-codegen-llvm/AGENTS.md` states the goal is an "identical failure set to cranelift", and the parity note predates the fix.

**Failure scenario.** `match b { true => A, _ => B }` on a `Bool` under `KESTREL_BACKEND=llvm`. Two-case True/False switches take a dedicated bool-branch path, but one-concrete-case-plus-default falls through to the general switch, emitting `Discriminant` on a `@guaranteed Bool`. LLVM issues `load i32` from a 1-byte slot; the compare against 1 sees 3 bytes of adjacent stack, so `b == true` takes the wildcard arm. The existing regression test has no `// backends:` header, so it only runs on the env-default backend.

**Fix.** Port the cranelift arm: bind `TypeRepr::Scalar(t)`, load at `t`, then truncate/zext to `disc_ty`. Factor the normalize into one `fn normalize_disc` used by both arms.

---

#### 9. `NominalCopySemantics`/`NominalStaticness` memos depend on a thread-local recursion stack that is not part of the cache key

**Category** incremental-hazard + global-state / **Severity** medium / **Locations**
`lib/kestrel-semantics/src/lib.rs:478`, `:484`, `:489`, `:503`, `:520`, `:525`, `:571`, `:695`
`lib/kestrel-semantics/src/staticness.rs:199`, `:228`, `:232`
`lib/kestrel-copy-fold/src/lib.rs:163`
`lib/kestrel-mir-lower/src/items/struct_lower.rs:49`
`lib/kestrel-hecs/src/query.rs:421`

**Evidence.** The key is `{ entity, root }` (lib.rs:478-482). The body pushes onto a thread-local, computes, pops (lib.rs:503-507). The recursive edge consults the *stack*, not the engine:

```rust
// lib.rs:520-528
fn query_nominal_semantics(ctx, entity, root) -> CopySemantics {
    if computing_contains(entity, root) {
        CopySemantics::Copyable            // cycle fallback
    } else {
        ctx.query(NominalCopySemantics { entity, root }).semantics
    }
}
```

`staticness.rs:199-234` is identical with a `Staticness::Static` fallback. The in-code justification (lib.rs:522-524, "the definition can't make itself non-copyable without another child already doing so") holds for *direct* self-reference and fails for a 2-cycle: the second nominal is memoized before the first one's own non-copyable child has been seen. The result is then stored as a normal memo (`insert_memo`, query.rs:421). The file's own WARNING (lib.rs:484-488) names the invalidation half but not the visit-order half.

There is a second defect in the same three lines: the fallback branch returns **without** calling `ctx.query`, so no `Dependency` edge is recorded — the poisoned memo is not even invalidated when the other cycle participant changes.

**Why it matters.** All five copy layers read this memo, and `lower_copy_behavior` stamps it straight onto `StructDef.type_info.copy` (struct_lower.rs:49), which decides bitwise-copy vs. move for every value of that type.

**Failure scenario.** With a user-defined conditionally-Copyable wrapper (`struct MyBox[T]: not Copyable { var p: Pointer[T] }` + `extend MyBox[T]: Copyable where T: Copyable`) and mutual recursion through its gating argument: `struct A { var b: MyBox[B]; var h: Handle }` / `struct B { var a: MyBox[A] }`. Querying `A` first memoizes `B = Copyable` (B's walk back to A hits the guard), while A correctly resolves `NotCopyable`. Every later consumer of `B` bit-copies a struct that transitively owns a `Handle`. Querying `B` first gives the correct answer for both. Demand order is deterministic per build but is decided by declaration order, so a file rename or reorder flips it.

**Fix.** Seed the in-progress entry with the *bottom* of the lattice (`NotCopyable` / `NotStatic`) so the fallback can only be conservative, or mark the result cycle-tainted and skip `insert_memo`. The holistic fix is a framework-level cycle-recovery hook (`QueryFn::cycle_fallback`) in `kestrel-hecs` so both thread-locals can be deleted and the dependency bookkeeping is handled once.

---

#### 10. Static-member lookup truncates to the first `extend` block

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-name-res/src/helpers.rs:96`, `:99`, `:118`
`lib/kestrel-name-res/src/resolve_value.rs:319`, `:416`, `:426`, `:571`, `:577`
`lib/kestrel-name-res/src/traversal.rs:92`
`lib/kestrel-name-res/src/type_members.rs:3`
`lib/kestrel-type-infer/src/resolve.rs:774`, `:795`
`lib/kestrel-hir-lower/src/expr.rs:372`

**Evidence.** `type_members.rs:3` declares `TypeMembers` the "Single source of truth for 'what members does this type have?'", and its walk merges *every* extension (traversal.rs:92-97). `kestrel-type-infer::resolve_static_member` independently accumulates across all extensions too (resolve.rs:795-806). But `find_in_extensions`, which is what `ResolveValuePath` actually uses, stops at the first hit:

```rust
// helpers.rs:110-120
for &ext in &extensions {
    let matches: Vec<Entity> = ctx.query(VisibleChildrenByName{parent: ext, ..}).into_iter().filter(...).collect();
    if !matches.is_empty() { return matches; }
}
```

Its own doc (helpers.rs:96) admits it: "Returns the matches from the first extension that has any (extensions are not merged across)" — a description, not a rationale, and not blessed in any AGENTS.md.

**Why it matters.** The resulting set becomes the *entire* overload set: `expr.rs:372-378` turns `ValueResolution::Overloaded` into `HirExpr::OverloadSet`, and a single `Def(fn)` callee is never re-widened by inference. Truncation at name resolution is unrecoverable downstream.

**Failure scenario.**
```kestrel
struct Foo {}
extend Foo { public static func make(a: Int64) -> Foo { ... } }   // file A
extend Foo { public static func make(b: String) -> Foo { ... } }  // file B
let x = Foo.make(b: "hi");
```
`find_in_extensions` returns only `[make(a:)]`, so inference reports "unknown argument label 'b'". The identical call reached as `T.make(b:)` with `T: SomeProto` bound to `Foo` goes through type-infer's implementation, sees both overloads, and compiles. Splitting one `extend` block into two — a pure refactor — silently breaks the call.

**Fix.** Have `resolve_extension_static_method` call `TypeMembersByName { .. }` and filter with `is_static_method`, making `TypeMembers` the actual owner for both crates. Same for `resolve_assoc_type_static_member`'s `children[0]` / `ext_members[0]`.

---

#### 11. The solver and the move checker ask `TypeParamCopyRequirement` with different `context`

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-semantics/src/lib.rs:289`, `:292`, `:591`
`lib/kestrel-type-infer/src/solver.rs:2528`, `:2530`, `:2652`
`lib/kestrel-type-infer/src/resolve.rs:2135`
`lib/kestrel-analyze/src/body/move_tracking.rs:1849`, `:1856`

**Evidence.** `TypeParamCopyRequirement::execute` builds its scan list from two roots — `parent_of(self.param)` (lib.rs:289) and the ancestor chain of `self.context` (lib.rs:292) — so `context` selects which enclosing where-clauses are visible, and it is part of the key. The two callers pass different things, and both label the line:

```rust
// solver.rs:2528-2530  — HOOK: structural parent context scopes the bound lookup.
let context = ctx.query_ctx.parent_of(entity).unwrap_or(entity);
// move_tracking.rs:1849-1856 — HOOK: body-owner context scopes the bound lookup.
context: self.mcx.cx.entity,
```

`HirCopyLayer` (lib.rs:591) is a third convention (caller-supplied). Inside `extend Pair[T] { … }`, `T` resolves to *Pair's* param (`resolve_extension_type_param`, resolve_name.rs:178-192), so `parent_of(T) == Pair` and the extension's where-clauses are never reached. `kestrel-semantics/docs/architecture.md:63` states the contract this breaks: one decision tree "so all layers agree on each instantiation." `InferCtx` already carries the body owner, marked `#[allow(dead_code)]` (ctx.rs:171).

**Failure scenario.** Inside `extend Set[T, H] where T: Cloneable, ...` (a live shape — `lang/std/collections/set.ks:1290`), a `Conforms(T, Cloneable)` obligation is answered by `solver_copy_class` scoped to `Set`, whose chain carries no Cloneable bound → `RequiresCopyable` → `type_conforms_copyable(want_cloneable=true) == false` → spurious `DoesNotConform` on legal code. That site compiles today only by accident: `elem.clone()` is a *member* lookup, which routes through `collect_param_protocol_bounds`/`body_owner` instead of a `Conforms` constraint.

**Fix.** Give `CopyLayer`/`StaticLayer` an explicit `bound_context()` hook and make every implementation supply the *asking* body/decl entity. `solver_copy_class` and `solver_ty_is_static` should pass `ctx.owner`; `WorldResolver::copy_semantics_of` should pass `self.body_owner`.

---

#### 12. Associated types on type params are matched by **name string** against ancestor where-clauses, and E439 can't see extension-target params

**Category** fragility / **Severity** medium / **Locations**
`lib/kestrel-name-res/src/resolve_type.rs:319`, `:326`, `:341`, `:403`
`lib/kestrel-analyze/src/decl/generics.rs:407`, `:416`
`lib/kestrel-ast-builder/src/builders/extension.rs:78`, `:160`

**Evidence.** `resolve_type_param_assoc` takes the type-param *entity*, extracts `tp_name` at resolve_type.rs:326, and thereafter passes only the string — walking the param's ancestors and then the *context's* ancestors (:341-347). The comparison is `if segments.len() != 1 || segments[0].name != type_param_name { continue; }` (:403).

The guard that would make this safe is E439 (shadowed type param), and it collects outer params solely from the `TypeParams` component on ancestors (generics.rs:416). `build_extension` never calls `build_type_parameters` — it sets `TypeParams` only via `introduce_rhs_free_type_params` (extension.rs:78), which returns early when the extension has no `Conformances` (extension.rs:83-86). So `extend Array[T]` carries no `TypeParams` and E439's walk finds nothing.

**Failure scenario.**
```kestrel
extend Array[T] where T: Iterable {
    func f[T](x: T) -> T.Item { ... }   // inner T has NO bounds
}
```
`ScopeFor` resolves `T` to `f`'s own param. `T.Item` walks `f` (no where clause) → the extension, whose `where T: Iterable` *subject string* is `"T"` → match → returns `Iterable.Item`. No E439 (extension has no `TypeParams`), no E440. The unbounded inner `T` is silently given `Iterable`'s associated type. There is zero test coverage — `rg E439 testdata/` returns nothing.

**Fix.** Compare entity identity: resolve the where-clause subject through `ResolveName` in the *bearing entity's* scope and compare the resulting `Entity`. Separately, have `check_type_param_shadowing` also collect the extension-target LHS param entities (`collect_lhs_target_names` already computes that set).

---

#### 13. A missing `;` after an expression statement in a function body is silently accepted

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-parser/src/block/mod.rs:666`, `:669`, `:297`
`lib/kestrel-parser/src/stmt/mod.rs:336`, `:348`
`lib/kestrel-parser/src/event.rs:179`
`lib/kestrel-lsp/src/handlers/completion.rs:577`

**Evidence.** Two encodings of "an expression statement needs `;`". Inline blocks (`if`/`while`/`for` bodies, match arms) reject it (`Err(Rich::custom(..., "expected semicolon"))`, block/mod.rs:297). Top-level blocks synthesize one:

```rust
// block/mod.rs:666-669
.or(empty().map_with(|_, e| Some((to_kestrel_span(e.span()), true)))),
...
Some((semi, _synth)) => Ok(BlockItem::Statement(StmtVariant::Expression(expr, semi))),
```

The `true` flag saying "synthesised" is discarded as `_synth`. The zero-width span is the crate's agreed marker for "synthesised" (event.rs:179-201 `add_token_or_missing`, which emits both an `expected \`;\`` error and a `Missing` wrapper node), and the sibling emitter honours it (`stmt/mod.rs:336`) — but the expression-statement emitter uses plain `sink.add_token` (stmt/mod.rs:348).

**Verified empirically.** `kestrel dump cst` on `func f() { doThing()\n doOther(); }` produces `Semicolon@102..102 ""` — a bare zero-width `Semicolon` **not** wrapped in `Missing` — and `dump diagnostics` produces nothing. The same body inside an `if` produces three errors. The LSP test at `completion.rs:573-599` writes the invariant down explicitly ("parser-synthesised closing tokens (`;`, `}`, `)`) must surface as `expected …` diagnostics") and covers every synthesis site *except* this one.

**Fix.** Change `emit_expression_statement` to `sink.add_token_or_missing(SyntaxKind::Semicolon, semicolon, ";")`. Better: make the semicolon an `Option<Span>` on both `StmtVariant::Expression` and `VariableDeclarationData`, so "absent" has one representation and every emitter must handle it.

---

#### 14. Closure lowering's `SavedState` hand-mirrors `OssaBodyCtx`; three per-body fields are unsaved

**Category** side-table / **Severity** medium / **Locations**
`lib/kestrel-mir-lower/src/body/closure.rs:77`, `:88`, `:337`, `:504`
`lib/kestrel-mir-lower/src/body/mod.rs:397`, `:2056`, `:2732`, `:3818`, `:3981`
`lib/kestrel-mir-lower/src/body/stmt.rs:118`
`lib/kestrel-mir-lower/src/body/call/mod.rs:192`, `:268`
`lib/kestrel-mir/src/body.rs:31`

**Evidence.** `SavedState` (closure.rs:77) is a hand-written duplicate of a *subset* of `OssaBodyCtx`'s fields, and its own doc states the invariant: `value_forwarding` is saved because "value IDs index the parent body's arena, so it must be swapped out for the closure's separate arena (otherwise a stale forwarding entry resolves into the wrong arena: out-of-bounds)" (closure.rs:88-91). Sixteen fields are swapped (closure.rs:337-354) and restored (:504-519).

`pending_writebacks: Vec<PendingWriteback>` (mod.rs:397) is not among them, and every field of `PendingWriteback` is a parent-arena `ValueId` (mod.rs:437-441). The closure body is lowered with the same `self`, and `lower_stmt` ends with an unconditional drain: `self.drain_writebacks(0);` (stmt.rs:118). That emits `Take(wb.slot_addr)` into the *closure's* body, and `emit_take` → `inherit_slot_taint` does `self.body.value(address).root` (mod.rs:2732) where `OssaBody::value` is a raw index (`&self.values[id.index()]`, kestrel-mir/body.rs:31). A writeback is provably pending across closure-argument lowering: `wb_mark` is taken at call/mod.rs:192 and the drain is at call/mod.rs:268, with argument lowering in between. `init_field_flags` and `field_inits` are likewise unsaved (currently inert only because `body_context` *is* reset).

**Failure scenario.** A get/set accessor member with no `mutating ref`, a Copyable element, in mutating-receiver position, with a closure-literal argument whose body has ≥1 statement — e.g. `struct Counter { var n: Int; mutating func apply(f: (Int)->Int) {...} }` then `var xs = [Counter(n: 1)]; xs(0).apply(f: { v in let d = v; d + 1 });`. `self.body.values[slot_addr.index()]` panics with index-out-of-bounds, or — if the closure body allocated enough values — silently writes back a garbage element and drops the real write.

**Fix.** Stop hand-mirroring: extract the per-body fields of `OssaBodyCtx` into a `BodyState` struct held by value, so `lower_closure_expr` does one `mem::replace` and adding a field is mechanically covered. As an immediate patch, add the three fields to all three blocks and `debug_assert!(self.pending_writebacks.is_empty())` at the end of `lower_closure_expr`.

---

### Diagnostics infrastructure

#### 15. Every inference error is rendered twice by two divergent tables; the dedup rule lives in consumers and the LSP has none

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-compiler/src/queries/infer.rs:36`, `:41`
`lib/kestrel-compiler/src/diagnostic.rs:83`, `:119`, `:229`, `:280`, `:299`, `:305`, `:320`, `:332`, `:343`
`lib/kestrel-analyze/src/body/type_check.rs:45`, `:63`, `:79`, `:211`, `:238`, `:245`, `:249`, `:275`
`src/main.rs:690`
`lib/kestrel-test-suite/src/compiler.rs:119`
`lib/kestrel-lsp/src/handlers/diagnostics.rs:97`, `:104`
`lib/kestrel-lsp/src/convert.rs:137`

**Evidence.** The same `typed.errors` list is rendered by two full per-variant matches in different crates: `ResolvedInferError::to_diagnostic` (diagnostic.rs:83, thrown at infer.rs:41) and `TypeCheckAnalyzer::format_error` (type_check.rs:79, stamped E100 at :63). Both anchor on `err.span()`, both are `Severity::Error`, and `vis_label` is duplicated verbatim (type_check.rs:275 vs. diagnostic.rs:119).

They have already drifted in text and in code:

| Variant | `diagnostic.rs` | `type_check.rs` |
|---|---|---|
| `ItWrongArity` | "requires single-parameter context" (:229) | "used in {expected}-parameter context" (:211) |
| `ConventionMismatch` | fixed sentence (:305) | `format!("convention mismatch: {}", detail)` (:249) |
| `OpaqueUnderlierNotCopyable` | "opaque return type hides a non-Copyable type" (:299) | `detail.to_string()` (:245) |
| `KindMismatch` / `RefFunctionAsValue` / `RefInTypeArgument` | `E624` / `E491` / `E492` (:320/:332/:343) | all `E100` |

The "these are duplicates" policy is then re-implemented per consumer, three ways: CLI `.filter(|d| d.descriptor_id != "E100")` (main.rs:690, whose comment names bug #209); the test harness, same filter plus a prefix-match dedup (compiler.rs:119-133); and the LSP, which pushes both streams into `grouped` with no filter at all (diagnostics.rs:96-109) and no filtering in `from_analyze` either.

**Failure scenario.** Open any `.ks` with `let x: Int64 = "s";`. The editor shows two ERROR squiggles on the identical range for every type error. For a closure-kind error it is worse: `error[E624]: closure kind mismatch...` and `error[E100]: closure kind mismatch...` — one mistake under two codes. Adding a new `InferError` variant requires arms in both tables; if the wording diverges, the harness's prefix dedup silently stops matching.

**Fix.** One owner. Add `fn code(&self) -> Option<&'static str>` / `fn message(&self, detail) -> String` as inherent methods on `InferError` in `kestrel-type-infer` (it already owns `span()`), make `to_diagnostic` a thin wrapper, and delete `type_check.rs::format_error` and the E100 duplicate entirely — the CLI and harness already treat the codespan stream as canonical.

---

#### 16. Diagnostic-code registry has no ownership or reachability enforcement

**Category** fragility / **Severity** low (aggregate) / **Locations**
`lib/kestrel-analyze/src/decl/generics.rs:94`, `:601`, `:603`; `lib/kestrel-analyze/src/compilation/type_annotation_resolution.rs:26`, `:123`
`lib/kestrel-analyze/src/body/closure.rs:85`, `:97`, `:322`, `:351`
`lib/kestrel-analyze/src/body/access_mode.rs:203`, `:204`, `:155`
`lib/kestrel-analyze/src/registry.rs:45`, `:50`, `:76`
`lib/kestrel-hir-lower/src/ty.rs:522`, `:595`
`docs/error-codes.md:3`, `:218`, `:344`, `:346`; `docs/language/pattern-matching.md:488`; `docs/language/modules.md:467`

Four independent instances of one gap — `docs/error-codes.md:3-8` claims "Every code below corresponds to a descriptor or emit site" and "Each code maps to exactly one diagnostic (enforced by a unit test)", but the only test (`registry.rs:76`) checks id *uniqueness across descriptor arrays* and nothing else:

- **One code, two meanings.** `E436` is declared `non_protocol_bound` (generics.rs:94) but the `TypeResolution::NotFound` arm hardcodes `descriptor_id: "E436"` with E476's exact message (generics.rs:601-603 vs. type_annotation_resolution.rs:26/:123). `struct Set[T] where T: NonExistent {}` prints `error[E436]: cannot find type 'NonExistent' in this scope`, and `docs/error-codes.md#e436` describes something else.
- **Registered but dead.** `E600` and `E602` have descriptors (closure.rs:85, :97) with zero emit sites — E600 moved to the solver (closure.rs:322-330) where it renders as E100, E602 is a `// TODO` (closure.rs:351). Both are documented as live with worked examples (`docs/error-codes.md:365`, `docs/language/closures.md:908`).
- **Positional aliases.** `const E210: usize = 5; const E499: usize = 6;` (access_mode.rs:203-204) index into a positional DESCRIPTORS array. The constant's *name* is the only thing tying it to the right code, and nothing checks it. Inserting a descriptor between E207 and E210 silently reassigns both codes, their severities, and their docs links.
- **Nine emitted codes are undocumented.** `E480`–`E487`, `E489` come from `hir-lower/ty.rs:522-534` and `:595`; `docs/error-codes.md`'s references section starts at E488. Meanwhile `docs/language/pattern-matching.md:488` and `docs/language/modules.md:467` present ~37 codes (`E05xx`/`E06xx`) the compiler has never emitted, including fabricated `error[E0601]:` transcripts.
- **Analyzer registry.** `find_body_check`/`find_decl_check` are linear first-match by `AnalyzerId` with no uniqueness assertion (registry.rs:45/:50), so a copy-pasted analyzer that forgets a new `AnalyzerId` variant is silently never run while its twin double-emits — the exact shape of bug the descriptor-uniqueness test was written for, one level up.

**Fix.** One test that walks every registered descriptor and asserts (a) it is reachable from some emit site or is on an explicit RESERVED allowlist, (b) every literal `descriptor_id: "E…"` belongs to its emitting analyzer's own array, (c) `AnalyzerId`s are unique across all three lists. Replace positional indices with the existing `fn descriptor(id: &str)` lookup helper (`exhaustiveness.rs:92`) that AGENTS.md:332 already recommends. Then make `docs/error-codes.md` generated, not written.

---

#### 17. Analyzer diagnostics bypass the world accumulator; `kestrel dump` drops them and exits 0

**Category** side-table / **Severity** low / **Locations**
`lib/kestrel-analyze/src/lib.rs:219`; `lib/kestrel-compiler-driver/src/lib.rs:164`, `:169`; `lib/kestrel-compiler/src/lib.rs:137`; `src/main.rs:198`, `:323`, `:364`, `:365`

Every other phase deposits into the hECS accumulator (`ctx.accumulate`), which `Compiler::diagnostics()` reads. Analyzers alone return a `Vec<AnalyzeDiagnostic>` parked on `AnalyzeSummary` (driver lib.rs:164). So "the diagnostics of this compilation" has two homes and each consumer must remember both. `kestrel build`, the LSP, and the test suite do. `kestrel dump` does not — `driver.analyze_all(false);` with the result unbound (main.rs:323), then emits and gates the exit code on the accumulator only (:364/:365). `DumpKind::Diagnostics`'s own doc claims "All accumulated diagnostics (lex, parse, infer, analyze)" (main.rs:198). A file whose only error is E430 prints nothing from `kestrel dump diagnostics` and exits 0.

**Fix.** Make `emit_diagnostics`/`has_errors` take the `AnalyzeSummary` so the type system forces both halves, or have `Analyze` also `ctx.accumulate` a converted `Diagnostic`.

---

#### 18. `kestrel dump mir` swallows the MIR-stage diagnostics it just accumulated

**Category** fragility / **Severity** low / **Locations** `src/main.rs:327`, `:330`, `:334`, `:364`, `:385`, `:388`, `:402`; `lib/kestrel-compiler/src/lib.rs:234`, `:242`, `:253`, `:276`

`lower_to_mir` accumulates the rich coded diagnostic (lib.rs:234) then returns an `Err` whose payload is only a count (lib.rs:242). `kestrel build` orders it correctly (`emit_diagnostics()` at main.rs:276 *before* consuming the result). `dump_mir`'s `?` at main.rs:327 returns before the sole emission point at main.rs:364, so E494–E496 print as `error: compilation failed with 1 error(s)` with the span, code and notes constructed and discarded. The default stage is the affected branch. Note the neighbouring cranelift-codegen arm *does* emit first (main.rs:352), so the omission is plainly unintentional. `lower_to_mir_stage`'s doc comment "Never aborts and never accumulates diagnostics" (lib.rs:253) is false.

---

### Query-framework and incremental hazards

#### 19. `TypedBody`'s hand-written `Hash` omits five output fields and hashes `errors` by length only

**Category** incremental-hazard / **Severity** medium / **Locations**
`lib/kestrel-type-infer/src/result.rs:32`, `:36`, `:39`, `:42`, `:45`, `:91`, `:93`
`lib/kestrel-type-infer/src/lib.rs:70`
`lib/kestrel-hecs/src/query.rs:54`, `:384`, `:398`, `:402`, `:461`
`lib/kestrel-compiler/src/queries/infer.rs:28`, `:40`
`lib/kestrel-analyze/src/lib.rs:148`; `lib/kestrel-analyze/src/body/type_check.rs:52`

**Evidence.** The substrate makes `Hash` of the output the *entire* change signal: `type Output: Clone + Hash` (query.rs:54), `let new_fp = Fingerprint::of(&result);` (query.rs:398), then backdating at query.rs:402-403. `InferBody::Output = Option<Arc<TypedBody>>`, and `TypedBody`'s manual `Hash` (result.rs:91-137) hashes six maps plus `self.errors.len()` (result.rs:93) — the **length only**. Never hashed: `field_subscripts` (:32), `promotions` (:36), `type_args` (:39), `errors` contents (:42), `error_details` (:45).

`InferWithDiagnostics::execute` records exactly one dependency (`ctx.query(InferBody{..})`, infer.rs:28) because `ctx.throw`/`accumulate` records none. So backdating `InferBody` means `InferWithDiagnostics` verifies clean, returns from cache, and never reaches `clear_for_query` (query.rs:384) — the stale accumulated diagnostic stays.

**Failure scenario.** LSP only (a batch build has one revision). File `b.ks` is not edited, so its entities are stable; a cross-file change (renaming a struct that `b.ks`'s error message mentions, or deleting a `private func` that `b.ks` calls) re-executes `InferBody(b.f)` and yields a *different* single error with different `error_details`, while `errors.len()`, `expr_types`, `local_types` and `resolutions` are byte-identical. Fingerprint matches → `changed_at` backdated → the editor keeps showing the old message and the old symbol name for the rest of the session.

**Fix.** Make the fingerprint mechanically total. Switch the map fields to `BTreeMap`/sorted `Vec` and `#[derive(Hash)]` so a 12th field cannot be forgotten. If a hand impl must stay, hash all five omitted fields and add a destructuring `const _: () = { let TypedBody { .. } = ...; };` assertion. Separately, `kestrel-hecs` should either require `Output: PartialEq` and compare values, or document at query.rs:54 that a partial `Hash` is a correctness bug, not an optimization.

---

#### 20. The LSP despawns *before* `begin_revision()`, erasing the only invalidation despawn produces

**Category** ordering-dependency / **Severity** medium / **Locations**
`lib/kestrel-lsp/src/compiler_worker.rs:311`, `:319`, `:342`, `:350`
`lib/kestrel-hecs/src/world.rs:184`, `:191`, `:203`, `:31`
`lib/kestrel-hecs/src/change.rs:60`
`lib/kestrel-hecs/src/query.rs:444`
`lib/kestrel-hecs/docs/architecture.md:29`

**Evidence.** `World::despawn` records invalidation only in the current revision's changed set (world.rs:191, and the former-parent mark at :196). `begin_revision` → `ChangeSet::advance` clears it (`self.changed.clear()`, change.rs:62). The invalidation path consults only that ephemeral set:

```rust
// query.rs:444-447
Dependency::Component { entity, .. } => { if self.changes.is_changed(*entity) { return false; } },
```

Notably, the revision-precise record `EntityRecord::last_changed` (world.rs:31) *is* faithfully maintained and read by `World::last_changed` (world.rs:203) — which has **zero callers anywhere**; `QueryContext` is never handed `entities`. Two records of "when did E change", and the invalidation path reads the one that gets wiped. The documented lifecycle is begin_revision → mutate → query (architecture.md:29-45); `sync_user` does the reverse (despawn at :311-316, `begin_revision()` at :319), and `rebuild_files` repeats it at :342/:350.

**Failure scenario.** File deletion (the case where nothing re-marks). `paths_to_unbuild` is non-empty, `paths_to_build` is empty. The despawn loop marks every declaration entity and the owning module changed; `begin_revision()` at :319 clears all of it; the build loop is empty. Every memo verifies clean and `ScopeFor` keeps serving a children list containing despawned entities — the exact failure `world.rs:757` (`despawn_invalidates_children_of_query`) was written to prevent, which only passes because it despawns *after* `begin_revision`. On a content edit the bug is masked by `set_parent` re-marking the module.

**Fix.** Make `EntityRecord::last_changed` the single owner: pass `&[EntityRecord]` into `QueryContext` and change query.rs:444 to `if self.last_changed(*entity) > since { return false; }`, mirroring the sub-query branch. That makes marks durable across revisions and deletes the unwritten rule. Until then, `debug_assert!(self.changes.changed_entities().is_empty())` in `begin_revision`, and move the two LSP call sites.

---

#### 21. `World::snapshot` clones query memos but resets the accumulator store — and both docs say the opposite

**Category** incremental-hazard / **Severity** medium / **Locations**
`lib/kestrel-hecs/src/world.rs:324`, `:335`, `:336`, `:533`
`lib/kestrel-hecs/src/accumulator.rs:30`, `:34`
`lib/kestrel-hecs/src/query.rs:171`, `:322`, `:384`
`lib/kestrel-hecs/docs/snapshots.md:26`, `:29`
`lib/kestrel-test-suite/src/lib.rs:36`, `:59`, `:103`, `:115`

**Evidence.** A query's output has two halves: the return value and its `ctx.accumulate` side effects. `snapshot()` copies one and discards the other, in adjacent lines:

```rust
// world.rs:335-336
queries: RefCell::new(self.queries.borrow().clone()),
accumulators: RefCell::new(AccumulatorStore::new()),
```

`QueryStorage::clone` copies both the memo stores *and* the verifier table (query.rs:171-183). Accumulated values are only re-emitted when a query re-executes (`clear_for_query` at query.rs:384), and a cache hit never executes — so in a snapshot world, every diagnostic belonging to a pre-warmed query is gone forever.

Three documents assert the opposite: the function's own doc comment ("Starts with fresh query caches and accumulators", world.rs:324), `snapshots.md:26` (queries listed under **Fresh**), and `snapshots.md:29` ("Query caches are intentionally dropped"). A unit test named `snapshot_preserves_query_cache` (world.rs:533) locks in the code behaviour, so the docs contradict both the code and its own test.

The cost is documented in-tree as a workaround: *"Diagnostics are emitted into the CACHE compiler's sink at first query execution; per-test compilers get memoized cache hits that never re-emit, so without this record a stdlib type error is invisible to every test and only surfaces as a mysterious downstream mono ICE"* (test-suite lib.rs:36-41), feeding `render_stdlib_errors` — which filters to `Error` severity only (lib.rs:103), so stdlib warnings are lost outright.

**Fix.** Give `AccumulatorStore` a deep clone (same pattern as `ComponentStore::clone` / `ErasedStore::clone_store`) and clone it alongside `queries`, so the two halves of a query's output travel together. Then correct world.rs:324 and snapshots.md:26-29, and delete the `StdlibCache::errors` side channel.

---

#### 22. Push/pop guards are not unwind-safe, and two hosts catch panics and keep the thread

**Category** global-state / **Severity** medium / **Locations**
`lib/kestrel-hecs/src/query.rs:363`, `:371`, `:378`, `:384`, `:389`, `:394`
`lib/kestrel-mir-lower/src/ty.rs:188`, `:275`, `:280`, `:289`, `:318`
`lib/kestrel-semantics/src/lib.rs:503`, `:506`
`lib/kestrel-semantics/src/staticness.rs:208`, `:211`
`lib/kestrel-compiler-driver/src/lib.rs:42`, `:49`
`lib/kestrel-lsp/src/compiler_worker.rs:137`
`lib/kestrel-test-suite/tests/file_tests.rs:160`

**Evidence.** Four instances of the same shape — push, do fallible work, pop — with no `Drop` guard:

| State | Push | Fallible work | Pop |
|---|---|---|---|
| `QueryContext::active` | query.rs:378 | `q.execute(self)` :389 | :394 |
| `OPAQUE_RESOLVE_STACK` | ty.rs:275 | `panic!("ICE: opaque type origin…")` :289; `ctx.query(InferBody)` :280 | ty.rs:318 |
| `COMPUTING_COPY_SEMANTICS` | lib.rs:503 | `nominal_copy_semantics_impl` | :506 |
| `COMPUTING_STATICNESS` | staticness.rs:208 | impl | :211 |

For the opaque stack, the `panic!` sits literally *between* the insert and the remove. Both panic-catching hosts keep the thread: `catch_unwind(AssertUnwindSafe(|| ctx.query(InferWithDiagnostics{..})))` inside `infer_all`'s per-body loop on one shared `ctx` (driver lib.rs:42/:49, whose doc says "one bad body doesn't abort the whole run"), and the LSP worker's single long-lived thread (compiler_worker.rs:137). The test harness catches per-`.ks` on a shared worker-pool thread (file_tests.rs:160).

**Why it matters.** The `active`-stack leak makes every later query touching that key die with a *fabricated* "Query cycle detected" (query.rs:363-374) — defeating the very isolation `catch_unwind` was added for — and `clear_for_query` at :384 has already discarded the failed query's diagnostics. The thread-locals are worse because they persist for the thread's life: a leaked `OPAQUE_RESOLVE_STACK` entry makes that origin resolve to `ty_arena.error()` forever, and a leaked `COMPUTING_COPY_SEMANTICS` entry makes that nominal answer `Copyable` forever. Entity ids restart at 0 per `Compiler`, and every test world is a snapshot of the *same* cached stdlib world, so a leaked key from test A names the same stdlib nominal in test B.

**Fix.** RAII guards everywhere: `struct ActiveGuard<'a>(&'a RefCell<Vec<ActiveQuery>>)` in `execute_query`, and an equivalent for each thread-local — ideally one shared `hecs::recursion_guard(&KEY, key)` helper so a fifth copy is not written by hand. Move `clear_for_query` to after a successful `execute`. Replace `ty.rs:289`'s `panic!` with `return ctx.module.ty_arena.error()` — fail-soft is the documented pipeline contract.

---

#### 23. Accumulated values are never pruned by revision or despawn

**Category** incremental-hazard / **Severity** low / **Locations** `lib/kestrel-hecs/src/accumulator.rs:34`, `:76`; `lib/kestrel-hecs/src/query.rs:294`, `:384`; `lib/kestrel-hecs/src/world.rs:184`, `:344`; `lib/kestrel-compiler/src/lib.rs:132`, `:182`

The only removal path is `clear_for_query(qk)` from the re-execution path. `despawn` never touches accumulators, and entity IDs are never reused — so a despawned entity's query key can never re-execute and its bucket is unreachable garbage for the process lifetime. `Compiler::diagnostics()` returns all buckets under the doc comment "Collect all diagnostics from the current revision" (lib.rs:132), which is false.

Two aggravating details. `QueryContext::accumulate` falls back to a `{type_id: 0, key_hash: 0}` sentinel when no query is active (query.rs:294) — structurally unreachable by the only pruning path, and MIR lowering uses it for real: `LowerCtx::new` builds a `QueryContext` outside any query (`mir-lower/context.rs:28`), so every E497/E503/ICE from lowering lands there permanently. `kestrel dump mir --stage all` runs `lower_module` ten times on one `Compiler` and prints each such diagnostic ten times. And in a long LSP session the store grows without bound while `World::accumulated` clones the whole vector on every refresh.

The UI is currently saved only incidentally: `FileMap::lookup` drops diagnostics whose `file_id` is not a live file. `TestCompiler` already hand-rolls a diff (`let before = self.compiler.diagnostics(); … .filter(|d| !before.contains(d))`, test-suite/compiler.rs:143-150) because of this.

**Fix.** Record `Revision` per bucket at `push` time and have `World::accumulated` return only buckets at or after `self.revision`; add a `clear_for_entity` invoked from `World::despawn`. `debug_assert!` on a push under the `{0,0}` sentinel.

---

### Parser and CST integrity

#### 24. `add_token_or_missing` widens a diagnostic span by one raw **byte**, producing non-UTF-8-boundary offsets that make the LSP drop every diagnostic

**Category** fragility / **Severity** medium / **Locations**
`lib/kestrel-parser/src/event.rs:189`, `:206`
`lib/kestrel-lsp/src/position.rs:74`
`lib/kestrel-lsp/src/convert.rs:58`
`lib/kestrel-lsp/src/handlers/diagnostics.rs:97`
`lib/kestrel-lsp/src/compiler_worker.rs:137`

```rust
// event.rs:188-196
let anchor_end = self.last_real_token_end().unwrap_or(span.end);
let diag_start = anchor_end.saturating_sub(1);
```

`anchor_end` is a char boundary; `anchor_end - 1` is only if the token's final character is single-byte. The lexer accepts full Unicode XID identifiers (`[\p{L}_][\p{L}\p{N}_]*`, `kestrel-lexer/src/lib.rs:423`, tested with `café`/`αβγ`/`_hello世界`). The span reaches the LSP unmodified and hits `let line_text = &self.text[line_start..offset];` (position.rs:74) — no `is_char_boundary` guard exists anywhere under `lib/`. The conversion runs in `refresh` (diagnostics.rs:97), *outside* the worker's `catch_unwind` (compiler_worker.rs:137), so nothing logs it.

**Failure scenario.** `func f() {\n    let x = café\n}` in an editor. The var-decl parser's recoverable semicolon yields a zero-width span; `add_token_or_missing` anchors on `café`; `diag_start` lands between the two bytes of `é`; `offset_to_position` panics; `publish_diagnostics` is never called. One mis-anchored span kills the entire publish for that edit, with no error message.

**Fix.** Anchor on a real token boundary — have `last_real_token_end` return a `Span` and emit over `anchor.start..anchor.end`. If a one-character widening is wanted, walk back with `char_indices`/`floor_char_boundary`. Independently, make `offset_to_position` clamp to a char boundary so no span can crash the server.

---

#### 25. Module/import emitters re-derive `.` `(` `)` `,` `as` spans by byte arithmetic — and it fires on shipping stdlib source

**Category** fragility / **Severity** medium / **Locations**
`lib/kestrel-parser/src/common/emitters.rs:65`, `:106`
`lib/kestrel-parser/src/import/mod.rs:183`, `:187`, `:202`, `:213`, `:229`, `:236`
`lib/kestrel-parser/src/common/parsers.rs:185`
`lib/kestrel-parser/src/event.rs:283`

The combinators match the punctuation and discard it (`separated_by(token(Token::Dot))`, `.ignore_then(...)`, `.then_ignore(...)`), and every emitter invents the span from a neighbouring identifier assuming exactly one byte of separator and zero trivia:

```rust
sink.add_token(SyntaxKind::LParen, Span::new(id, last_segment_end + 1..last_segment_end + 2));
let as_start = name_span.end + 1;
```

But `token()`/`identifier()` are trivia-skipping wrappers, so `import A.B. (X, Y)` and multi-line imports are grammatical. `TreeBuilder` then takes token text from the fabricated range and hits the non-trivia safety net in `emit_trivia_until`, emitting the real punctuation as `SyntaxKind::Error`.

**This is not hypothetical.** `lang/std/numeric/int64.ks:7-17` is a multi-line `import std.core.(\n ... \n)`; the `RParen` span is computed as `last_item_end..last_item_end + 1`, which is the newline, so the CST gets an `RParen` whose text is `"\n"` plus a spurious `Error` token holding the real `")\n"`. The same shape recurs across `lang/std/numeric/*.ks` and `lang/std/iter/iterator.ks`. `tree.text()` still round-trips, so the crate's round-trip assertions pass. Nothing user-visible breaks today only because every consumer guards on `Error` — but `Error` is the documented recovery marker, so emitting it for well-formed source violates `kestrel-parser/docs/architecture.md:21-28` ("preserve source token order and source spans").

**Fix.** Return the real separator spans from `module_path_parser_internal` and `import_declaration_parser_internal` instead of reconstructing them, and have `EventSink::add_token` `debug_assert!` that the emitted text matches the kind's expected lexeme.

---

#### 26. Escape-sequence decoding is implemented three times, and one copy silently drops `\u` entirely

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-hir-lower/src/literal.rs:22`, `:236`
`lib/kestrel-hir-lower/src/pat.rs:575`, `:580`, `:586`, `:632`, `:719`, `:783`
`lib/kestrel-hir-lower/src/expr.rs:212`
`lib/kestrel-ast-builder/src/lower.rs:527`, `:3093`

Three independent decoders of one language rule:

1. `decode_string` (literal.rs:22) — full table, returns errors as data (`Vec<EscapeError>` → E700-E703), rejects over-long `\u{}` at literal.rs:236.
2. `unescape_char_content` (pat.rs:632) — re-implements the table for char literals, accumulates diagnostics inline **with no E-code**, and reads hex with an unbounded `for c in chars.by_ref() { if c == '}' { break; } hex.push(c); }` (pat.rs:719) — no close-brace check, no digit limit. So `"\u{00000041}"` is rejected while `'\u{00000041}'` compiles to `'A'`. It also diverges on unknown escapes and on `\x80`.
3. `unescape_char_simple` (ast-builder/lower.rs:3093) — used for the literal segments of any string containing `\(` (lower.rs:527). It has **no `\x` and no `\u` arm and no error path**, and nothing re-decodes downstream.

Plus an asymmetry within (2): pattern position calls `parse_char` (pat.rs:575) which passes `None` for diagnostics and `unwrap_or(0)`, while expression position calls `parse_char_validated` (pat.rs:586) with `Some(ctx)`.

**Failure scenarios.** `"\u{41}"` yields `"A"` but `"\u{41} \(x)"` silently yields the literal text `u{41} ` with zero diagnostics — a wrong-value miscompile. And `match c { '\u{D800}' => handleSurrogate(), _ => other() }` produces `HirPat::Literal(Char(0))` with no diagnostic (the surrogate branch is `ctx`-gated), so the arm silently matches NUL, while the same literal in an expression errors correctly.

**Fix.** One `decode_escape(chars) -> Result<u32, EscapeErrorKind>` in `literal.rs` owning the whole table including `\u` digit-count and range rules; both other sites call it and differ only in what they do with the scalar. Route char-literal errors through the same `EscapeError` data path so `StringEscapeAnalyzer` assigns E700-E703 to all three forms.

---

#### 27. `SyntaxKind`'s 258 variants are restated twice in the same file; a miss reads back as `Error`

**Category** single-source-of-truth / **Severity** low / **Locations** `lib/kestrel-syntax-tree/src/lib.rs:37`, `:339`, `:349`, `:355`, `:743`, `:1006`, `:1010`

The write direction is derived and total (`Self(kind as u16)`, :349). The read direction is 258 hand-written `const NAME: u16 = SyntaxKind::Name as u16;` declarations plus 258 hand-written match arms over `raw.0`, ending in `_ => SyntaxKind::Error,` (:1006). Because the scrutinee is `u16`, rustc cannot check it, and there is no round-trip test. I verified all three lists currently agree at 258/258/258 — no live divergence. The contrast is `impl From<Token> for SyntaxKind` (:355), an exhaustive enum match that is self-maintaining. Note that parser tests *do* go through `kind_from_raw` (`event.rs:249` `build()`), so a new kind that ships with a parser test asserting its kind would be caught; the hole is a new kind with no such test.

**Fix.** Derive `TryFrom<u16>` (or a `syntax_kinds!` macro generating enum + both tables from one list), or add a test that loops `0..=LAST` asserting `kind_from_raw(kind_to_raw(k)) == k`.

---

### Remaining single-source-of-truth duplication

#### 28. `lang.*` intrinsics are declared by a cross-product loop and lowered from a hand-written table; 105 names have no lowering

**Category** single-source-of-truth / **Severity** medium / **Locations**
`lib/kestrel-ast-builder/src/lang_module.rs:284`, `:389`, `:522`
`lib/kestrel-mir-lower/src/body/call/intrinsic.rs:18`, `:1135`, `:1335`, `:1344`, `:1351`
`lib/kestrel-mir-lower/src/body/call/mod.rs:26`
`lib/kestrel-mir-lower/src/items/function_sig.rs:151`
`lib/kestrel-mir/src/mono/collect.rs:302`; `lib/kestrel-mir/src/mono/verify.rs:389`
`lang/std/numeric/int8.ks:284`

Producer: `seed_integer_ops`/`seed_float_ops`/`seed_cast_ops` generate 367 public, `Vis::Public`, name-resolvable, type-checkable functions by string formatting. Consumer: a 228-row hand-written `static TABLE` matched by exact string with `?` on a miss. I re-derived the diff mechanically: **105 seeded names are unreachable** — every `i1_*` arithmetic name, `i8_bswap`, and 69 `cast_*` pairs (`cast_i64_u32`, `cast_u8_f64`, `cast_i8_f64`, …). Drift runs both ways: `"ref_to_ptr"` (intrinsic.rs:1135) is never seeded, so that row is dead.

The asymmetry is the tell: `i16_bswap`/`i32_bswap`/`i64_bswap` are lowered, `i8_bswap` is not — and `lang/std/numeric/int8.ks:284` hand-writes `self` with a doc comment still claiming it is "lowered to a `bswap` intrinsic", while `int16.ks:285` uses the intrinsic.

**Blast radius correction:** this fails *loudly*, which is why it ranks here rather than higher. `mono/collect.rs:302` skips body-less callees, so `rewrite_callee` leaves `Callee::Direct` and `mono/verify.rs:389` raises a spanned `Diagnostic::bug()` naming the intrinsic. Still an ICE where a diagnostic belongs, for a surface the compiler itself advertises.

**Fix.** Export the intrinsic rows (name, `Op`, arity, param/return types) from one shared table and have `seed_lang_module` iterate it. Minimum viable: a test that walks a freshly seeded world's `lang` children and asserts every `Intrinsic` function is lowerable, with matching arity.

#### 29. `@builtin(.X)` arguments are never validated; `Builtin`'s four hand-maintained tables already have a hole

**Category** single-source-of-truth / **Severity** low / **Locations** `lib/kestrel-hir/src/builtin.rs:75`, `:117`, `:339`, `:567`, `:725`, `:781`; `lib/kestrel-name-res/src/resolve_builtin.rs:46`; `lib/kestrel-analyze/src/compilation/unknown_attribute.rs:73`; `lang/std/core/literals.ks:173`

`Builtin` is described by four parallel tables; three are exhaustive matches rustc checks, the fourth (`from_attribute_name`) is a string match ending in `_ => None`. `DefaultArrayLiteralType` has a variant, a `name()` arm and a `kind()` arm, and the stdlib annotates it — but no `from_attribute_name` arm. I diffed all 142 `@builtin(.X)` spellings in `lang/` against the arms: it is the sole mismatch (`Bool` is the only other armless variant). `EntityBuiltin` returns bare `None` with no diagnostic, and `unknown_attribute.rs` only allowlists attribute *names*, never arguments.

No behaviour changes today because `ResolveBuiltin` tries name-based resolution first and `name()` returns `"Array"`. The real cost is that `builtin.rs:496-516` explicitly relies on the attribute index being authoritative for swappable bindings, so a typo in a `@builtin` argument yields a dead lang item with no signal anywhere.

**Fix.** One `&'static [(Builtin, &str, BuiltinKind)]` table all three accessors derive from, plus a test asserting `from_attribute_name(attr_name(b)) == Some(b)` for every variant. Emit a diagnostic when `@builtin`'s argument is unparseable.

#### 30. `desugar_logical_and` re-encodes the `SHORT_CIRCUIT_OP_PROTOCOLS` row and silently drops the RHS

**Category** single-source-of-truth / **Severity** medium / **Locations** `lib/kestrel-hir/src/body.rs:724`; `lib/kestrel-hir-lower/src/desugar.rs:45`, `:111`, `:116`, `:119`, `:125`; `lib/kestrel-hir-lower/src/expr.rs:1325`; `lib/kestrel-hir-lower/src/stmt.rs:201`; `lib/kestrel-hir-lower/AGENTS.md:6`

`&&` goes through the blessed table (`lookup_short_circuit_op`, desugar.rs:45). Comma-chained conditions (`if a, b` / `guard a, b`) go through a hand-rolled twin that re-states the protocol, method name and label inline (desugar.rs:111/:116/:119) — and on failure returns bare `lhs`, discarding the RHS closure with no diagnostic (desugar.rs:125-126), where the table-driven path emits `emit_missing_operator_diagnostic` + `HirExpr::Error`. The crate's own AGENTS.md:6-24 states the rule this violates. Both functions take already-lowered `HirExprId`s and build the same closure, so one is a drop-in for the other.

**Fix.** Delete `desugar_logical_and`; have `lower_if_conditions` call `self.desugar_binary_hir(BinaryOp::And, ...)`.

#### 31. `lower_condition_chain` re-invokes `on_fail` per condition, duplicating diagnostics and lowering `else if` chains exponentially

**Category** fragility / **Severity** medium / **Locations** `lib/kestrel-hir-lower/src/expr.rs:1215`, `:1220`, `:1224`, `:1552`, `:1593`, `:1618`, `:1653`

`lower_condition_chain` recurses one level per condition and calls `let fail = on_fail(self);` at **every** level (expr.rs:1593 and :1618), where `on_fail` is a full `lower_block`/`lower_expr` of the else body. The doc comment (expr.rs:1552-1557) argues only that duplication is *semantically* safe; it says nothing about node count or diagnostics. Nothing dedups downstream.

`if let .Some(a) = x, let .Some(b) = y { … } else { notAFunction(); }` reports "undefined name" twice on the same span. Add an `else if let` chain and the inner chain is lowered twice, the final else four times — 2^depth, with type errors, mutability errors and every HIR-walking analyzer's findings multiplied to match.

**Fix.** Lower `on_fail` to a single `HirExprId` at the top of `lower_condition_chain` and thread that id through the recursion — which is already the semantics the doc comment describes.

#### 32. `substitute_resolved_ty` is a second, non-exhaustive copy of the substitution kernel

**Category** single-source-of-truth / **Severity** low / **Locations** `lib/kestrel-mir-lower/src/ty.rs:315`, `:513`, `:563`; `lib/kestrel-mir/src/substitute.rs:24`, `:79`; `lib/kestrel-type-infer/src/result.rs:180`, `:192`; `lib/kestrel-type-infer/src/compare.rs:254`; `docs/references-prototype/references-plumbing.md:265`

`kestrel_mir::substitute` is deliberately exhaustive with an explicit `MirTy::Ref` arm and no `_ =>`. `substitute_resolved_ty` (ty.rs:513) ends in `_ => ty.clone()` (ty.rs:563), silently swallowing `ResolvedTy::Ref` and `ResolvedTy::Opaque`. `compare.rs:254`'s `contains_error` walks the same enum exhaustively, so the `_ =>` is the outlier. The crate already depends on the exhaustive kernel. Note the fix is *already filed as an unchecked TODO* in `references-plumbing.md:265` ("add ResolvedTy::Ref arms to substitute_resolved_ty"). I could not construct a shipped program that reaches it, so this is a robustness gap rather than a live ICE — but the `_ =>` guarantees any future `ResolvedTy` variant is skipped instead of being a compile error.

#### 33. Small duplicated predicates and magic strings

**Category** single-source-of-truth / **Severity** low — grouped because they share one cause and one fix shape.

| What | Copies | Locations | Risk |
|---|---|---|---|
| Trivia-kind set (`Whitespace \| Newline \| LineComment \| BlockComment`) | 9 | `syntax-tree/utils.rs:34`, `parser/event.rs:42`, `common/parsers.rs:34`, `ty/mod.rs:154`, `block/mod.rs:28`+`:767`, `stmt/mod.rs:101`, `declaration_item/mod.rs:163`+`:235`, `expr/mod.rs:1311` | splitting `///` out of `LineComment` silently deletes doc comments from the CST |
| `is_type_node` (14 variants) vs `is_type_kind` (12 — no `TyRef`/`TyMutRef`) | 2 | `ast-builder/ast_type.rs:257`, `builders/helpers.rs:509` | already diverged; masked only by the parser's `Ty > TyRef` double-wrap (`ty/mod.rs:854`) |
| Root entity = the magic name `"<root>"` | 4+ | `compiler/lib.rs:57`, `name-res/visibility.rs:91`, `mir-lower/name.rs:20`, `doc/lib.rs:627`, `lsp/ty_format.rs:108` | `Vis::Internal` fails **open** — every `internal` decl becomes universally visible with no signal |
| `parent_is_type` (`Struct\|Enum\|Protocol\|Extension`) | 7 | `builders/function.rs:42`, `field.rs:101`, `subscript.rs:39`, `hir-lower/expr.rs:530`, `analyze/static_context.rs:59`, `duplicate_symbol.rs:125`, `lsp/hover.rs:69` | adding `NodeKind::Class` requires 7 lockstep edits; a miss means "cannot use 'self' in static method" on correct code |
| `member_lookup_name` (init/subscript sentinel) | 3, keyed on **different components** | `name-res/helpers.rs:75` (`Subscript` marker), `conformance_completeness.rs:1455` + `parent_protocol_conformance.rs:207` (`NodeKind`) | drift has already happened elsewhere: `witness_lower.rs:717` documents that `Callable` is insufficient and checks `Computed` instead |
| Operator→`BinaryOp` map vs. parser's accepted-token list | 2 + `unwrap_or(BinaryOp::Add)` / `unwrap_or(UnaryOp::Neg)` fallbacks | `ast-builder/lower.rs:853`, `:915`, `:2217`, `:2230`; `parser/expr/operators.rs:21`, `:51` | currently unreachable (`From<Token> for SyntaxKind` is exhaustive and forces a new kind), but the fallback silently rewrites the operator if it ever is |
| `BinaryOp`→source text | 2, **already wrong** | `kestrel-ast/pretty.rs:741-744` (`and`/`or`/`..=`) vs `hir-lower/desugar.rs:1372-1375` (`&&`/`\|\|`/`...`) | the wrong copy is the one in the user-facing diagnostic (desugar.rs:1321) |
| Stdlib location precedence chain | 4, all different | `src/main.rs:607`, `lsp/lib.rs:542`, `test-suite/lib.rs:129`, `test-suite/runner.rs:144`; plus `io/libc_shims.c` written 3× | the test-suite copies skip the `exists()` check, turning a stale `KESTREL_STD` into a silent zero-file stdlib load |
| Type-sugar `[T]`/`T?`/`[K:V]` binding | inert `@builtin(.*TypeOperator)` lang items + hardcoded name strings | `hir-lower/ty.rs:132`, `:754`, `:828`; `hir/builtin.rs:241`, `lang/std/collections/array.ks:1988` | the annotation a stdlib author would edit is read by nothing; resolution uses `context: owner` (user scope) rather than root |
| `BuiltinKind::Protocol`'s `requires_fields_conform` / `tuple_conformance_propagation` | declared, **zero readers** | `hir/builtin.rs:21`, `:23`, `:788`; hardcoded to `Builtin::FFISafe` at `protocol_field_conformance.rs:47` | the analyzer's own module doc (`:4`) advertises the data-driven rule that does not exist |
| kestrel-doc forks `kestrel_ast::pretty::format_type` | 2, both print `Never` where the language writes `!` | `doc/signature.rs:551`, `:588`; `ast/pretty.rs:644`, `:683`; contrast `lsp/ty_format.rs:92` | `docs/stdlib/std.core.md:5188` publishes `-> Never`, a signature that does not parse |

---

### Verifier and self-check gaps

#### 34. The OSSA verifier's linear-ownership check is block-local, and never runs after mono at all

**Category** fragility + ordering-dependency / **Severity** medium / **Locations**
`lib/kestrel-mir/src/verify.rs:6`, `:161`, `:172`, `:305`, `:417`, `:513`, `:1205`, `:1259`
`lib/kestrel-mir/src/passes/mod.rs:140`
`lib/kestrel-compiler/src/lib.rs:357`, `:373`, `:441`
`lib/kestrel-mir/src/mono/verify.rs:41`
`lib/kestrel-mir/AGENTS.md:9`, `:11`

**Evidence.** AGENTS.md:11 promises "Linear ownership (@owned consumed exactly once)". The algorithm rests on an unchecked assumption stated at verify.rs:6: *"no fixpoint needed because the block-parameter live-in contract guarantees each block can be verified in isolation."* Nothing verifies that contract — `check_operands_defined` collects definitions across *every* block (verify.rs:172-181), so using a value defined elsewhere without threading it is accepted. Then:

```rust
// verify.rs:417-421
None => {
    // Value not tracked in this block — likely defined elsewhere.
    // We still flag it so the caller sees it.
    true
},
```

The comment says it flags; the code pushes no error and reports success. `BlockVerifier.owned` is fresh per block (verify.rs:305), and Check 2 scans only that block-local map (verify.rs:1205). So leaks are caught and cross-block double-consumes are invisible — exactly the hazard `kestrel-mir-lower/AGENTS.md:44-61` warns about ("Resolve held values through `value_forwarding` before consuming them"), and the reason `memory_model/deinit/aggregate_{control_flow,try}_field_no_double_drop.ks` exist.

AGENTS.md:9 additionally claims `verify_ossa` "runs after lowering and after mono passes." It does not — an exhaustive grep finds no post-mono call site, `run_pipeline_until` is `debug_assert!`ed pre-mono, and `monomorphize_mir` runs four ownership-rewriting passes (copy-prop, cross-block copy-prop, `mark_independent_takes`, `expand_destroy_copy`) followed only by `verify_mono`, which checks layouts/concreteness/callees and has no ownership state machine. AGENTS.md is internally inconsistent: its pass-pipeline section names `verify_mono` as the post-mono check.

**Fix.** Add a `check_block_local_defs` pass requiring every operand to be defined by that block's params or an earlier instruction in it; then make the `None` arms of `try_consume_exempting` and `assert_live` push a `VerifyError`. Separately, either run `verify_ossa` over `MonoModule` at `Stage::Expand` (factor the two `MirModule` reads behind a trait) or correct AGENTS.md:9 and add the ownership machine to `verify_mono`.

#### 35. Borrow mutability lives only on the instruction, so threading a mut borrow through a block param downgrades it to shared

**Category** side-table / **Severity** low / **Locations** `lib/kestrel-mir/src/value.rs:96`; `lib/kestrel-mir/src/block.rs:7`; `lib/kestrel-mir/src/verify.rs:529`, `:705`, `:775`, `:875`; `lib/kestrel-mir-lower/src/body/mod.rs:1760`

`ValueDef` records only `borrow_source: Option<ValueId>`; the mut-vs-shared distinction exists solely as the opcode. When a borrow enters a block as a `@guaranteed` param, the verifier reconstitutes it with a hardcoded `is_mut: false` (verify.rs:705, and again at :875 for the Op1 forward). Check 5, the only exclusivity check, filters on `info.is_mut`. Threading is real, not hypothetical — `rebind_scope_values` stamps the successor param's `borrow_source` with the comment "the new block's @guaranteed param IS the same borrow continued". Kestrel deliberately has no exclusivity rule, so this is a lost internal sanity check rather than a user-facing gap — hence low.

#### 36. Mono's `WitnessCache` is built at collection cost, discarded with `let _ =`, and keyed by a lossy pair

**Category** side-table / **Severity** low / **Locations** `lib/kestrel-mir/src/mono/collect.rs:29`, `:52`, `:386`; `lib/kestrel-mir/src/mono/mod.rs:63`, `:77`, `:702`; `lib/kestrel-mir/src/mono/witness.rs:451`

The cache is populated during BFS collection by a **second** full linear scan of the witness slice (`resolve_witness_call` already calls `find_witness_with_method` internally at witness.rs:451), returned as half of `CollectionResult`, and then `let _ = witness_cache;`. Nothing ever reads `.resolved`. Its key `(protocol, self_type)` discards both the protocol's type arguments and the method — which `find_witness_with_method` *does* discriminate on — so `Convertible[Int]` and `Convertible[String]` on one type collapse to one slot, last-writer-wins. The `let _ =` signals intent to wire it up; whoever does inherits a key the rest of the compiler already knows is insufficient.

#### 37. `find_inherited_assoc_type` recurses through protocol inheritance with no cycle guard its sibling has

**Category** fragility / **Severity** medium (fails **loudly**) / **Locations** `lib/kestrel-name-res/src/resolve_type.rs:499`, `:628`, `:651`, `:671`; `lib/kestrel-name-res/src/resolve_name.rs:222`, `:226`, `:233`

`resolve_inherited_protocol_member` guards, with a comment saying exactly why: "`visited` guards against self/mutual protocol-inheritance cycles; without it, `protocol Foo: Foo` (or longer cycles) would stack-overflow here." `find_inherited_assoc_type` is the same walk with no `visited` and an unconditional self-call, and `resolve_ctx = parent_of(scope).unwrap_or(scope)` converges to root, so the arguments become fixed-point identical. `protocol A: B { type Item }` / `protocol B: A {}` plus `func f[T](x: T) -> T.Missing where T: A` is a SIGSEGV instead of E459 + E440. Existing cycle testdata passes only because it never projects an associated type. Guarding is the crate norm (`protocol_cycles.rs:106`, `conformance_completeness.rs:601`, `conformances.rs:284`), so this is an unforced omission. Ranked below silent defects because it crashes visibly, and only on already-invalid programs.

---

### Editor and tooling

#### 38. `disk_line_indices` is a second copy of file text that `didOpen`/`didChange`/`didClose` never update

**Category** side-table / **Severity** medium / **Locations** `lib/kestrel-lsp/src/server.rs:26`, `:38`; `lib/kestrel-lsp/src/lib.rs:156`, `:402`, `:420`, `:431`; `lib/kestrel-lsp/src/handlers/diagnostics.rs:87`, `:88`; `lib/kestrel-lsp/src/position.rs:14`, `:67`

`ServerState` names `sources` the single source of truth and then keeps a parallel path-keyed `HashMap<String, LineIndex>` where `LineIndex` owns a full second copy of the text. Only `load_workspace` and `did_change_watched_files` maintain it; the editor-event handlers touch `docs` + `sources` only. The publisher relies on the pair agreeing: `doc_indices.get(path).or_else(|| disk_indices.get(path))` (diagnostics.rs:87), and `offset_to_position` clamps silently rather than failing (position.rs:67).

Edit a workspace file, close the tab without saving: `did_close` drops the `OpenDoc` but deliberately keeps the edited buffer in `sources` (lib.rs:431-433), `refresh()` compiles the edited buffer, and the stale on-disk index maps every diagnostic to the wrong line. Second consequence the finder missed: a file opened via `did_open` that the workspace walk never visited has *no* disk entry, so after `did_close` the `or_else` yields `None` and every diagnostic in that file is silently dropped. `position.rs:1-6` claims the opposite intent ("Single source of truth for offset math").

**Fix.** Delete `disk_line_indices`; give `ServerState` a `fn line_index(&self, path)` that prefers `docs` and otherwise builds from `sources` on demand (or memoizes inside `set_source`, already the single mutation point).

#### 39. Completion open-codes member lookup instead of `TypeMembers`, missing every protocol-extension member

**Category** single-source-of-truth / **Severity** medium / **Locations** `lib/kestrel-lsp/src/handlers/completion.rs:283`, `:288`, `:298`, `:311`; `lib/kestrel-name-res/src/type_members.rs:3`, `:57`; `lib/kestrel-name-res/src/extensions.rs:150`; `lib/kestrel-lsp/src/handlers/signature_help.rs:243`

`push_members_for_type` implements steps 1 and 2 of `TypeMembers`' documented three, with a self-admitted gap comment (`// Protocol conformances aren't expanded here; M3 keeps it simple.`, completion.rs:298) and no visibility check at all. Signature help in the same crate uses the canonical `TypeMembersByName`. So completion and signature help disagree from the same cursor position. This is not hypothetical: `extend Comparable { lessThan, isAtLeast, isAtMost, isBelow }` and `extend Equatable { equal, notEqual }` (`lang/std/core/protocols.ks:104`, `:152`, `:184`) are invisible to `.`-completion on any conforming concrete type, and `private` members are offered across module boundaries.

#### 40. The two backends' `classify_named` disagree on a newtype over an aggregate field

**Category** single-source-of-truth / **Severity** medium / **Locations** `lib/kestrel-codegen-llvm/src/ty.rs:282`; `lib/kestrel-codegen-cranelift/src/ty.rs:220`, `:222`; `lib/kestrel-codegen-llvm/src/abi.rs:40`; `lib/kestrel-codegen-cranelift/src/abi.rs:33`; `lang/std/io/error.ks:154`

LLVM nests the checks and returns `Aggregate { size, align }` for a newtype over a non-scalar field (with a comment naming `IoError`); cranelift merges them with `&&`, so that case falls through to integer-by-size and returns `Scalar`. Its comment still asserts the superseded rule. `git log -S` shows the LLVM branch entered in the typed-ptr migration (6b0b64c8) with no cranelift counterpart. The divergence propagates straight into the ABI: `Scalar → Direct` / `Aggregate → Sret`, and `Consuming + Scalar → ByVal` / `Consuming + Aggregate → ByRef`. `struct IoError { var kind: IoErrorKind }` over a payload-carrying enum is `Sret`/`ByRef` under LLVM and `Direct(I64)`/`ByVal` under cranelift. I diffed the rest of both `classify` functions — they are otherwise identical, so this is the sole divergence.

#### 41. Atomic RMW width is hardcoded `I64` in cranelift but taken from the operand in LLVM

**Category** single-source-of-truth / **Severity** low / **Locations** `lib/kestrel-mir/src/op.rs:181`; `lib/kestrel-codegen-cranelift/src/inst.rs:885`, `:892`; `lib/kestrel-codegen-llvm/src/inst.rs:1010`, `:1019`; `lib/kestrel-ast-builder/src/lang_module.rs:686`

`Op::AtomicAdd`/`AtomicSub` are the only arithmetic ops carrying no `IntBits` payload, so each backend recovers the width independently — cranelift hardcodes `ir::types::I64`, LLVM uses the rhs. `lang.atomic_add` is seeded generic over an **unbounded** `T`, no MIR verifier check exists, and the only testdata pins `lang.ptr[lang.i64]`. A one-line stdlib declaration at `lang.ptr[lang.i32]` builds under LLVM and either fails cranelift's verifier or performs a 64-bit RMW over 4 adjacent bytes. Adding `AtomicAdd(IntBits)` to the op would break both backends' patterns and force the fix — the mechanism the rest of the `Op` table already relies on.

#### 42. `unsafe impl Sync for StdlibCache` is unsound

**Category** global-state / **Severity** medium (test harness only) / **Locations** `lib/kestrel-test-suite/src/lib.rs:45`, `:48`, `:49`, `:51`, `:115`; `lib/kestrel-test-suite/src/compiler.rs:51`; `lib/kestrel-hecs/src/world.rs:328`, `:335`; `lib/kestrel-test-suite/tests/file_tests.rs:44`, `:186`

```rust
// Safety: ... only accessed via world().snapshot() which clones all data into
// a fresh, independent World. No concurrent mutation occurs — the cached
// Compiler is read-only after init.
unsafe impl Sync for StdlibCache {}
```

The argument is wrong on its own terms: `World::snapshot(&self)` does `self.queries.borrow().clone()` (world.rs:335), and `RefCell::borrow` is a non-atomic RMW on the cell's borrow counter. `World` is `!Sync` precisely because of these `RefCell`s (and `VerifierFn`'s unbounded `Arc<dyn Fn>`); the `unsafe impl` is what permits the shared `&'static`. libtest-mimic 0.8.2 defaults to `available_parallelism()` workers, nothing in the repo passes `--test-threads`, and every stdlib test reaches `test_compiler(true)`. This is a data race in the Rust memory model. It cannot affect a shipped compilation, which is why it is medium and not high — but the symptom (a corrupted borrow flag → `BorrowMutError` panics reported as failures of unrelated `.ks` files) is exactly the unexplainable-flake class that costs days.

**Fix.** `OnceLock<Mutex<StdlibCache>>` and take the lock across `snapshot()`, or change `World::snapshot` to `&mut self` so the compiler rejects the shared-static access outright.

#### 43. Smaller tooling defects

| Finding | Locations | Note |
|---|---|---|
| `Compiler::build` is call-once-per-entity but nothing enforces it | `compiler/lib.rs:105`, `:162`, `:182`; `src/main.rs:547`, `:94` | `kestrel build main.ks main.ks` reuses the entity and runs `build_declarations` twice, duplicating every declaration. Fails loudly (E474/E426), but with labels pointing at the single source line. Make `build` despawn owned decls first. |
| `PARAM_COUNTER` is a process-global counter whose doc claims it is reset | `ast-builder/builders/params.rs:88`, `:90`, `:127` | Only two references exist; nothing resets it. The name reaches `AstParam.name` → `HirBody.locals` (hashed) and into E613/E611 message text (`default_param_ordering.rs:77`, `extern_ffi_safe.rs:150`) and MIR debug names. The correct pattern is one crate over: `desugar_opaque_params`' function-local `let mut opaque_index = 0u32;` (`function.rs:243`). |
| Diagnostic **message text** is built by iterating a `std::HashSet` | `analyze/body/initializer.rs:146`, `:196`, `:229`, `:444`, `:487`; `extension_conflict.rs:64`; `duplicate_callable.rs:144` | `struct P { let x: Int64; let y: Int64; init() {} }` prints `...: 'x', 'y'` or `...: 'y', 'x'` run to run. Latent only because all three E007 fixtures have exactly one field. `BTreeSet`/`IndexMap` fixes it. |
| `module.witnesses` tail is appended in `HashMap` order | `mir/passes/clone_shim.rs:120`, `:145`, `:174`; `mir/lib.rs:209`; `mono/witness.rs:304` | Order is load-bearing for `select_most_specific` and two first-match loops. Currently safe only because one shim exists per nominal. One-word fix to `IndexMap`. |
| A type-blind copy of the irrefutability rule survives in analyze | `analyze/body/refutable_pattern.rs:106`, `:114`, `:117`; `for_loop_pattern.rs:74`, `:111` | Its only caller is a `let…else` fallback that can still push hard E301 on degraded input, where both sibling analyzers skip. The crate's own AGENTS.md:321 forbids exactly this. E301 has zero test coverage. |
| Type-arg conformance failures deduped by a rendered display string | `type-infer/solver.rs:2007`, `:2009`, `:2074`, `:2135`; `ctx.rs:296`; `result.rs:412` | The key comment says "keyed structurally"; the key is `(String, Entity)` from a bare `Name` with no module path. `Format` (quill) vs `Format` (datetime) and `ParseError` × 2 already collide in-tree. Drops a diagnostic; does not affect soundness. |
| `ConformingProtocolInstantiations` dedup key embeds source spans | `name-res/conformances.rs:79`, `:103`, `:204`; `ast/ast_type.rs:91`, `:99` | The doc says dedup is by `(protocol, type_args)`; the key includes `Span`, so it never fires for parameterized conformances. **Do not "fix" by stripping spans** — they are currently the only thing distinguishing two conformance sources with identical arg spelling. Add `source` to the key instead. |
| mir-lower accumulates E497/E503/ICE into a sentinel bucket | `mir-lower/validate.rs:98`, `body/mod.rs:2023`, `:3284`, `call/args.rs:263`; `hecs/query.rs:291` | `lower_module` is not a query, so `accumulate` files under `{0,0}` which `clear_for_query` can never target. `kestrel dump mir --stage all` prints each such diagnostic 10×, and `lower_to_mir_stage`'s "never accumulates diagnostics" doc (compiler/lib.rs:253) is false. |

---

## Systemic Themes

**1. A semantic fact is decided once and re-derived by hand N times.** This is the dominant pattern and explains findings 3, 6, 7, 10, 11, 15, 26, 28, 29, 33, 39, 40, 41 — and the near-misses in 12 and 36. The tell is a comment that *names* the other copy ("mirrors the layout collection in struct_lower.rs", "Mirrors the discrimination in `lower_field_access`", "Mirrors `kestrel_ast::pretty::format_type`") without any mechanism enforcing the mirroring. Three of those comments are already false. The fix shape is always the same: make the owner `pub`, delete the copy.

**2. Degraded answers return silently instead of erroring.** Findings 1, 5, 13, 27, 28, 30, 32, 33 (operators), 34 all share the shape `if let Some(x) = lookup() { … }` / `unwrap_or(default)` / `_ => Error` / `find(...)?` where the failure path produces well-formed output that means something else. Kestrel's own contributing docs say "fail soft, never panic", and this is the pathological reading of that rule: fail soft *and* fail invisibly. The cure is to distinguish "this input is legitimately absent" (return a marker the caller must handle) from "my table is incomplete" (ICE).

**3. Hand-maintained parallel tables the compiler cannot check.** `SyntaxKind` ×3 in one file (27), `Builtin` ×4 with a hole (29), intrinsic seeder vs. lowering table with 105 diverged names (28), `InferError` ×2 renderers with drifted text and codes (15), descriptor arrays indexed positionally (16). In every case Rust *could* enforce the correspondence — exhaustive enum matches, a single `&'static [...]` iterated by both sides, a `TryFrom` derive — and in every case a `u16` scrutinee, a string key, or an array index defeats it.

**4. Manual state save/restore across boundaries that can unwind.** Findings 14, 22, and the `SavedState`/`OssaBodyCtx` split. Four thread-locals and one query stack use push/compute/pop with no `Drop`, in a process where two hosts catch panics and reuse the thread, and where entity ids repeat across `Compiler` instances. One localized ICE becomes a cascade of unrelated failures in later work.

**5. Two records of the same fact, where the load-bearing consumer reads the wrong one.** `ChangeSet::changed` (ephemeral) vs. `EntityRecord::last_changed` (revision-precise, zero callers) — finding 20. `queries` cloned vs. `accumulators` reset in `snapshot()` — finding 21. `sources` vs. `disk_line_indices` in the LSP — finding 38. `Local::span` read as a container by hover and as an identifier by rename — finding 2. Each is one field away from being structurally impossible.

---

## Recommended Order of Attack

**Tier 0 — fix now, small and self-contained, prevents data loss or wrong code.**

1. **LSP rename (#2).** Add `Local::name_span`, or at minimum reject synthetic spans and re-derive the identifier span from the CST. This currently deletes user code; it is the only finding with irreversible consequences. Low risk — `Target::Local` is untested today, so add the tests as part of the fix.
2. **Pattern-matching range specialization (#1).** Split int/char constructors into disjoint intervals in `compile_matrix`. Moderately invasive inside the crate but fully contained; the crate has good unit coverage. **Add execution tests, not just diagnostics tests** — the current `overlapping_ranges.ks` is diagnostics-only, which is exactly why this shipped.
3. **`break`/`continue` closure boundary (#5).** Three lines: save/restore `loop_labels` in `lower_closure` and `loop_break_tys` in `gen_closure`, then turn the MIR fall-throughs into ICEs. Zero risk.
4. **LLVM Bool discriminant (#8).** Port the cranelift arm verbatim, and add `// backends: llvm, cranelift` to `bool_match_with_wildcard_default.ks`.
5. **`unsafe impl Sync` (#42).** Wrap in a `Mutex` — three lines. Removes a class of unexplainable suite flake.
6. **Missing `;` (#13).** One-line change to `emit_expression_statement`; the invariant is already written down as an LSP test.

**Tier 1 — mechanical deduplication, low risk, high leverage.**

7. **`StoredInstanceFields` query (#3).** Write it once, route all eight sites through it. The only judgement call is which predicate is canonical — see "needs a decision" below. Do the `emit_struct_construct` name-binding change at the same time; it converts every future drift in this area from a silent shift to a hard error.
8. **Resolve `Copyable`/`Cloneable` via lang item in MIR (#6).** Add two `Option<Entity>` fields to `MirModule`, populate in mir-lower, replace five `ends_with` calls.
9. **Delete the E100 renderer (#15).** Move code+message onto `InferError` in `kestrel-type-infer`; `to_diagnostic` becomes a wrapper; delete `type_check.rs::format_error` and the three per-consumer filters. This one change removes ~190 lines and simultaneously fixes the LSP double-squiggle.
10. **`find_in_extensions` (#10), `desugar_logical_and` (#30), `lower_condition_chain` (#31), `is_type_kind` (#33), operator tables (#33), `BinaryOp` text (#33).** All are "delete the copy, call the owner."

**Tier 2 — framework-level, invasive, needs care.**

11. **RAII guards everywhere (#22).** Mechanical but touches the query engine's hot path; add the `ActiveGuard` first and verify `query_exec_count` is unchanged on a full build.
12. **`TypedBody` fingerprint (#19) and `AccumulatorStore::clone` (#21).** Both are correctness fixes to the incremental substrate. The `TypedBody` fix should switch to `BTreeMap` + `#[derive(Hash)]` so it cannot regress; that is a wider diff than hashing five more fields but is the only version that stays fixed.
13. **`begin_revision` ordering (#20).** *Prefer the structural fix* (read `EntityRecord::last_changed` from `QueryContext`) over moving the two LSP call sites. Moving the call sites is a one-line patch that leaves the trap armed for the next mutator. The structural fix touches `deps_unchanged`, so it wants careful before/after memo-hit counts.
14. **Intrinsic table unification (#28).** Shared `&'static [IntrinsicEntry]`. Invasive across two crates; the interim (a test asserting every seeded name is lowerable) is cheap and catches all future drift, so land that first.

**Tier 3 — needs a maintainer decision, not a mechanical fix.**

- **Which "stored field" predicate is canonical?** `!Static && !Computed` is my recommendation, but it requires the ast-builder to guarantee `Callable ⟹ Computed` (today `{ get set }` breaks that), and it changes what `NominalCopySemantics` folds over. Pick deliberately.
- **`TypeParamCopyRequirement`'s `context` (#11).** Three layers use three conventions. The right answer is "the asking body/decl entity" everywhere, but that changes which where-clauses are visible to the solver, which will move diagnostics. Needs a decision plus a full suite run.
- **`Local::span` semantics (#2).** Container or identifier? Adding a second field is the safe answer, but if hover is the odd one out, the cheaper fix is to change hover.
- **Cycle recovery in `kestrel-hecs` (#9).** The two thread-local guards exist because the framework panics on cycles. A `QueryFn::cycle_fallback` hook would let both be deleted and would fix the missing dependency edge as a side effect — but it is a real framework feature, not a patch.
- **`ConformingProtocolInstantiations` key (#43).** The obvious fix (strip spans) is *wrong* — it would collapse distinct conformance sources. Someone who knows the witness model must decide the right key.
- **Documentation ownership.** `docs/error-codes.md` claims completeness it does not have, four rival code tables exist, and `snapshots.md`, `mir/AGENTS.md:9`, `hecs/world.rs:324`, `lower_to_mir_stage`'s doc, and `params.rs:88` all describe behaviour the code does not have. The generated-not-written approach is the only durable answer.

**Explicitly risky:** #1 (pattern specialization) and #12 (name-string assoc-type matching) both change *which* program is accepted, so both will move diagnostics in the suite. Do them one at a time with a full triage run each. #3 changes struct layout collection, which is the most load-bearing table in the compiler — land it behind a run that diffs MIR output before/after on the stdlib.

---

## Coverage & Limits

**Audited by reading:** `kestrel-hecs`, `kestrel-parser`, `kestrel-syntax-tree`, `kestrel-lexer`, `kestrel-ast-builder`, `kestrel-hir`, `kestrel-hir-lower`, `kestrel-name-res`, `kestrel-semantics`, `kestrel-copy-fold`, `kestrel-type-infer`, `kestrel-analyze`, `kestrel-pattern-matching`, `kestrel-mir`, `kestrel-mir-lower`, both codegen backends, `kestrel-compiler`, `kestrel-compiler-driver`, `kestrel-lsp`, `kestrel-doc`, `kestrel-test-suite`, `src/main.rs`, and the relevant `AGENTS.md` / `docs/architecture.md` files in each. Everything under `target/`, `target-linux/`, `.claude/worktrees/`, `external/`, and `node_modules/` was excluded, and no worktree path is cited.

**Verification method.** Every finding here survived an adversarial second pass that re-read the cited lines; seven of the ninety candidates were refuted outright and are not reported. Before writing, I independently re-read the files behind eight of the top-ranked findings (pattern-matching specialization, LSP rename, the OSSA verifier, closure `break` lowering, `StdlibCache`/`World::snapshot`, the stored-field predicates, `closure_box` init selection, and the E100 dedup sites). All held. Two line numbers were off and are corrected in this report: `lower_break`'s silent fall-through is at `control.rs:351` (not 349), and `generate.rs`'s memberwise field filter spans `:1338-1344`.

**Not audited.** I did not build or run the compiler, run the test suite, or execute any `.ks` program — the one exception is that I invoked the already-built `kestrel dump cst`/`dump diagnostics` binary read-only to confirm the missing-semicolon finding (#13) and the `Error`-token behaviour in #25. No performance profiling was done. Cranelift and LLVM instruction selection was diffed only where a finding pointed at it, not systematically. The `lang/` stdlib was searched for triggering shapes but not reviewed for its own correctness.

**What a human should double-check.**
- **Whether findings 1, 5, and 8 reproduce at runtime.** All three are read-derived; each needs a five-line `.ks` execution test to confirm. If any does not reproduce, I have missed a guard.
- **The 105-name intrinsic diff (#28).** I re-derived it mechanically from the seeder loops, but the seeder has several conditional skips; a direct enumeration against a built world would be authoritative.
- **Severity of #3(b) and #6.** Both depend on user-written shapes (`static let` of a move-only type; a user protocol whose name ends in "Copyable") that no in-tree code currently uses. If those shapes are ruled out by policy, both drop a level.
- **The `TypedBody` fingerprint scenario (#19).** I am confident the `Hash` is incomplete and that the mechanism is live, but the exact cross-file edit that changes an error's identity while leaving every hashed map byte-identical is delicate — I traced it rather than reproducing it in a session.
- **Anything I graded "low" for lack of a current trigger** — the `SyntaxKind` tables, the trivia predicates, `parent_is_type`, the `<root>` sentinel. All are correct today. They are reported because the cost of each is one forgotten edit, and several have a fail-*open* mode (`Vis::Internal` in particular).

**Two things I looked at and want to explicitly reassure on.** The copy-fold kernel (`kestrel-copy-fold`) genuinely is a single decision tree with an exhaustive-match rule the layers obey — the one apparent MIR divergence (`FnKind::Mutating`) turns out to be documented, self-consistent with `needs_drop`, and pinned by a shipped test. And `kestrel-hecs`'s `QueryKey` is not a second identity decision: it is derived from the memo key by exactly one function, and every one of the 59 query key types in the tree derives `Hash` structurally, so the typed and erased maps cannot currently disagree.