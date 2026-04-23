# Test Failures — 2026-04-21

Run: `file_tests --test-threads=1` (full suite) on `feature/incremental-hecs`.
Result: **2761 passed · 57 failed** (2026-04-21, later same-day: after `kestrel-semantics` crate extraction, new `conformance_rules.rs` (E422/E423/E424), `exhaustive_return.rs` match-arm tail-expression handling, and `conformance_completeness.rs` two-pass associated-type-binding search that prefers qualified bindings like `Equal.Output = Bool` over unqualified siblings). stdlib/* stays clean (0 failures). 21 entries moved to `test-errors-fixed.md` (all 8 exhaustive-return items, 8 Cloneable/negative-conformance items, the 4 `NominalCopySemantics` query-cycle regressions via a new thread-local cycle guard, and the inherited-assoc-type `struct_conforming_to_child_provides_associated_type`). 13 new failures opened under "Spurious E001 on control-flow tails", "E458 on inherited associated types via method/where-clause binding", "Cycle analyzers over-eager on type-parameter bounds", "Parent-conformance analyzer false positive", and "Inference `could not infer type` on valid code".

> **Agent instructions:** When you fix a failing test (or verify that an existing entry has become passing), move it to `test-errors-fixed.md`. Move the full bullet — the `[x]` marker, the failure mode, and any explanation — preserving its subsection heading for context. If a subsection's last remaining item is being moved, move the subsection heading and its explanatory prose with it. `[x]` entries must never sit in **# False Negatives** or **# Stdlib** — those lists are for still-failing `[ ]` items only. Do not modify a test's source to make it pass; if a test is genuinely invalid (wrong syntax, etc.), note that in the entry.

---

# False Negatives

Compiler fails to emit a diagnostic for code that should be rejected.

## Move / ownership / use-after-move checks not running

Borrow/move-checker not executing or not wired into bind→infer→validate pipeline.

> **lib1:** emitted by `kestrel-semantic-tree-binder/src/body_resolver/move_tracker.rs` + `diagnostics/move_tracking.rs`, `diagnostics/deinit.rs`, `diagnostics/copy_semantics.rs` (move-tracking runs inside body resolution, per-branch join state).

- [ ] `memory_model/deinit/deinit_statement_marks_variable_as_moved.ks` — **expected:** `moved`

## Overload resolution / ambiguity not diagnosed

Wrong-arity / wrong-label calls produce generic "wrong number of arguments" instead of the richer "no matching overload" the tests want; multiple ambiguity cases surface no error at all.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/type_inference/diagnostics.rs` ("no matching overload", "ambiguous") — overload scoring happens in `kestrel-semantic-type-inference` and the analyzer reports the verdict. `extension_conflict/` handles cross-extension ambiguity; `duplicate_callable/` catches duplicate signatures at declaration time.

- [ ] `declarations/protocol_method_linking/ambiguous_method_satisfies_multiple_protocols.ks` — **expected:** `ambiguous`
- [ ] `declarations/associated_types/ambiguous_associated_type_without_qualification.ks` — **expected:** `ambiguous associated type`
- [ ] `types/generics/constraint_enforcement/wrong_labels_on_constrained_call.ks` — **expected:** `wrong argument label` · **got:** `no member 'calculate' on type 'T'`

## Cloneable / Copyable / `not` negative-conformance rules

Structural parent-requires-Cloneable check still missing: a struct/enum that stores a `Cloneable`-only field (a type that conforms to `Cloneable` but not the transitive `Copyable` closure) should itself opt into `Cloneable`. The current E502 analyzer uses the old lax "`Cloneable && not Copyable`" pair test, which stdlib types transitively satisfy, so the rule never fires. The stricter `NominalCopySemantics` query is available in `kestrel-semantics` but wiring it into E502 trips on the stdlib's `Array`/`String` containers whose owners never declared `Cloneable`; that's a separate stdlib-migration task.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/disallowed_conformance/`, `cloneable_field/`, `builtin_marker_protocol/`.

- [ ] `memory_model/cloneable/enum_with_cloneable_payload_without_conformance_errors.ks` — **expected:** `Cloneable`
- [ ] `memory_model/cloneable/struct_with_cloneable_field_without_conformance_errors.ks` — **expected:** `Cloneable`

## `let <refutable-pattern> = …` must be rejected

Refutable patterns in non-destructuring `let` produce only the downstream non-exhaustive-match error, not the required refutable-binding error.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/refutable_pattern/` (rejects refutable patterns in `let` bindings) — paired with `irrefutable_pattern/` for the `if let`/`while let` side.

- [ ] `patterns/let_destructuring/refutable_enum_pattern_error.ks` — **expected:** `refutable`
- [ ] `patterns/let_destructuring/refutable_literal_pattern_error.ks` — **expected:** `refutable`
- [ ] `patterns/let_destructuring/tuple_with_refutable_is_refutable.ks` — **expected:** `refutable`

## `if let` / `while let` binding scope leaks

Bindings introduced by `if let`/`while let` still resolve outside the scope.

> **lib1:** scoping is enforced during binding in `kestrel-semantic-tree-binder/src/body_resolver/patterns.rs` + `statements.rs` (pattern bindings pushed onto a scope stack that pops when the arm/loop body ends); lookups outside the scope fall through to the "undefined name" path in `body_resolver/paths.rs`.

- [ ] `patterns/if_let/scoping/binding_not_visible_after_if_let.ks` — **expected:** `undefined`
- [ ] `patterns/if_let/scoping/binding_not_visible_in_else.ks` — **expected:** `undefined`
- [ ] `patterns/while_let/scoping/binding_not_visible_after_loop.ks` — **expected:** `undefined`

## Init field-coverage across control flow

Initializer that sets `self.x` only in some branches should be rejected; check isn't flow-sensitive.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/initializer_verification/` (all-fields-assigned check) + `definite_assignment/` (flow-sensitive join of then/else/match arms, loop bodies). Diagnostics in `initializer_verification/diagnostics.rs`.

- [ ] `validation/initializers/init_only_in_while_body.ks` — **expected:** `does not initialize all fields`
- [ ] `validation/initializers/match_not_all_arms_initialize.ks` — **expected:** `does not initialize all fields`
- [ ] `validation/initializers/only_then_branch_initializes.ks` — **expected:** `does not initialize all fields`

## `for x in nonIterable` — missing Iterable conformance diagnostic

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/for_loop_pattern/mod.rs` — checks the subject of `for` for Iterable conformance and reports if missing. Array-element-type mismatches for the inference variant fall through to the conformance analyzer.

- [ ] `patterns/for_loops/for_loop_over_non_iterable.ks` — **expected:** `Iterable`
- [ ] `inference/mod/infer_array_element_type_mismatch.ks` — **expected:** `does not conform to protocol`

## Syntax Sugar Errors

Desugarings (`for`, `try`, operators, etc.) fall through to raw member-lookup errors (`no member 'iter'`, `no member 'next'`) instead of emitting the intended protocol-conformance diagnostic.

- [ ] `patterns/for_loops/for_loop_over_non_iterator_without_iter_method.ks` — **expected:** `Iterable` · **got:** `no member 'iter' on type 'NotIterable'`, `does not conform to protocol: ? !: Iterator`, `no member 'next' on type '?'`
- [ ] `expressions/control_flow/try_on_non_tryable_type.ks` — **expected:** `tryExtract` · **got:** `.Err not found on ?`, `could not infer type` (desugar cascade)

## Optional type diagnostics

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/type_check/` + `type_assignability/` — Optional-promotion rules live in the assignability check, and `does not conform` for the `null`-to-non-Optional case comes out of `conformance/diagnostics.rs` (ExpressibleByNilLiteral).

- [ ] `types/optional/incompatible_type_no_promotion.ks` — **expected:** `type mismatch`
- [ ] `types/optional/nested_optional_no_promotion.ks` — **expected:** `type mismatch`
- [ ] `types/optional/non_optional_type_cannot_be_null.ks` — **expected:** `does not conform`

## Pointer / type-argument diagnostics

> **lib1:** `kestrel-semantic-tree-binder/src/diagnostics/type_resolution.rs` — emitted while resolving a type reference; missing/empty type args on `Pointer[…]` are caught when the type is lowered.

- [ ] `types/pointer/lang_ptr_empty_brackets_error.ks` — **expected:** `type argument`
- [ ] `types/pointer/lang_ptr_without_type_args_error.ks` — **expected:** `type argument`

## Struct arity / label diagnostics

> **lib1:** `kestrel-semantic-tree-binder/src/diagnostics/struct_init.rs` (paired with `body_resolver/calls.rs` for memberwise-init arg checking). Produces the specific "has N field(s)" / "label" wording.

- [ ] `declarations/structs/wrong_arity_too_few.ks` — **expected:** `has 2 field(s)`
- [ ] `declarations/structs/wrong_arity_too_many.ks` — **expected:** `has 2 field(s)`
- [ ] `declarations/structs/wrong_label_name.ks` — **expected:** `label`

## Struct vs protocol same name

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/duplicate_symbol/` — wording is "duplicate" (lib2 says "already defined as a struct" via E425 which is close but not the expected phrasing).

- [ ] `validation/misc/struct_and_protocol_same_name_errors.ks` — **expected:** `duplicate` · **got:** E425 `'Foo' is already defined as a struct` (close semantics, wrong wording)

## Field-access / tuple-index diagnostics

Tests expect specific phrasing ("cannot index into non-tuple type", "out of bounds"); compiler emits the generic member-lookup error instead.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/field/` + `kestrel-semantic-tree-binder/src/diagnostics/member_access.rs` — member-lookup sees the receiver's type and emits the specific phrasing. Tuple-index-out-of-bounds / non-tuple-index are in the same member-access path (tuple arity known statically).

- [ ] `expressions/field_access/member_access_on_primitive_type_error.ks` — **expected:** `cannot access member on type`
- [ ] `validation/type_checking/tuple_index_on_non_tuple.ks` — **expected:** `cannot index into non-tuple type`
- [ ] `validation/type_checking/tuple_index_out_of_bounds.ks` — **expected:** `out of bounds`

## Self in wrong context

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/static_context/` (+ `diagnostics.rs`) — detects `self` references from inside free functions / static methods, with the specific "cannot use 'self' in …" wording.

- [ ] `expressions/calls/method_calls/self_in_free_function_error.ks` — **expected:** `cannot use 'self' in free function`
- [ ] `expressions/calls/method_calls/self_in_static_method_error.ks` — **expected:** `cannot use 'self' in static method`

## `self.init(...)` outside of an initializer body

No analyzer in lib2 detects delegating-init calls from non-init contexts. Currently falls through to ordinary member resolution and emits a label/E203 cascade.

- [ ] `declarations/delegating_initializers/delegation_outside_init.ks` — **expected:** diagnostic mentioning `init` (delegating init only valid inside another init)
- [ ] `declarations/delegating_initializers/delegation_to_nonexistent_init.ks` — **expected:** `no method 'init' on type 'Bad'` · **got:** generic `could not infer type` (init-resolution miss falls through to inference). **Regression:** previously fixed (different got-diagnostic `wrong number of arguments`); now fails via a different code path — see entry in `test-errors-fixed.md`.

## Primitive-method name-hint

> **lib1:** emitted by the method-resolution path in `kestrel-semantic-tree-binder/src/body_resolver/members.rs` / `calls.rs` with wording supplied by `analyzers/type_inference/diagnostics.rs` — when the receiver is a primitive, the resolver looks up the known-primitive method and produces the "must be called…" hint.

- [ ] `expressions/calls/method_calls/primitive_methods_errors.ks` — **expected:** `primitive method 'toString' on 'I64' must be called`

## `@platform(...)` exclusion — excluded decls should be dropped from name resolution

When a function/struct has `@platform(...)` that doesn't match the current target, lib1 drops the decl from binding so later references produce an "unknown name" diagnostic. lib2 leaves the decl reachable (or emits the generic inference error instead of an unknown-name diagnostic).

- [ ] `attributes/platform/non_matching_platform_function_excluded.ks` — **expected:** `excluded` (unknown function) · **got:** `could not infer type`
- [ ] `attributes/platform/non_matching_platform_struct_excluded.ks` — **expected:** `ExcludedStruct` (unknown type) · **got:** `could not infer type`

## Closure implicit-`it` parameter misuse

`{ it }` only has an implicit `it` binding when the expected closure type has exactly one parameter. Zero- or multi-param contexts should reject references to `it`.

- [ ] `expressions/closures/it_used_multi_param_context_error.ks` — **expected:** `it` (diagnostic about `it` in multi-param context) · **got:** `could not infer type`
- [ ] `expressions/closures/it_used_zero_param_context_error.ks` — **expected:** `it` (diagnostic about `it` in zero-param context) · **got:** `could not infer type`

## Protocol-extension method not visible when constraint not satisfied

`extend Filterable where Self: Sortable { func combined() {} }` should make `combined()` unavailable on a `Filterable` that doesn't also conform to `Sortable`. Instead of a "method not found / constraint not satisfied" diagnostic, we surface a generic inference error.

- [ ] `declarations/extensions/unconstrained_protocol_extension_not_found_when_constraint_not_met.ks` — **expected:** `member` (e.g. `no member 'combined'`) · **got:** `could not infer type`

## Struct-pattern unknown field

Field name in a struct pattern (`Point { x, z } => ...`) that isn't on the struct should be rejected at pattern binding.

- [ ] `patterns/pattern_types/struct_pattern_unknown_field_error.ks` — **expected:** `z` (unknown field) · **got:** `could not infer type`

## Compound assignment to non-lvalue

`5 += 1;` should emit a "left-hand side is not assignable" diagnostic; nothing is reported.

- [ ] `statements/compound_assignment/cannot_compound_assign_to_literal.ks` — **expected:** any error · **got:** no diagnostic

---

# False Positives

Compiler rejects valid code or emits spurious diagnostics where none should fire.

## E458 on inherited associated types via method/where-clause binding

The `conformance_completeness.rs` two-pass binding search fixed most `Equal.Output = Bool` cases by preferring qualified bindings, but some inherited-protocol paths still miss the default binding. Extension `extend IntIter: Iterator { type Item = lang.i64; func next() -> Item }` and qualified module path `std.num.Int64` in impl return types don't line up with the protocol requirement's abstract `Optional[Self.Item]` / `Item` placeholder.

- [ ] `declarations/associated_types/where_clause_resolves_method_type_param_from_caller_body.ks` — **expected:** no errors · **got at line 24:** E458 `method 'next' has wrong return type for protocol 'Iterator'` (impl return `Item` doesn't unify with expected `Item` when method-level type param is involved)
- [ ] `patterns/for_loops/for_loop_over_iterator_is_iterable.ks` — **expected:** no errors · **got at line 19:** E458 `method 'next' has wrong return type for protocol 'Iterator'` (qualified `std.result.Optional[std.num.Int64]` return doesn't structurally match protocol's `Optional[Item]`)

## Parent-conformance analyzer false positive

- [ ] `declarations/associated_types/struct_conforming_to_refined_protocol_must_satisfy_constraint.ks` — **got at line 11:** E421 `'BadIterator' conforms to 'SortedIterator' but not its parent protocol 'Iterator'` (the struct does provide the parent protocol's requirements — the parent-conformance analyzer walks the refinement chain incorrectly)

## Inference `could not infer type` on valid code

Introduced 2026-04-21 by the `type-infer/src/generate.rs` changes (skip-tail-coerce + `instantiate_entity_with_args` signature change). Generic enum type-arg defaulting and pointer cast expressions no longer resolve.

- [ ] `declarations/enums/generic_enum_with_explicit_type_args.ks` — **got at line 10:** `could not infer type`
- [ ] `types/pointer/cast_ptr_in_generic_context.ks` — **got at line 15:** two `could not infer type` errors on the cast expression
- [ ] `types/pointer/cast_ptr_with_untyped_ptr_null.ks` — **got at line 7:** two `could not infer type` errors on `ptr_null()` / cast
- [ ] `types/pointer/cast_ptr_with_various_primitives.ks` — 14 `could not infer type` errors across lines 7/11/15/19/23 on `cast[T]` through generic context
