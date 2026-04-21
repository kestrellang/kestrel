# Test Failures — 2026-04-21

Run: `file_tests --test-threads=1` (full suite) on `feature/incremental-hecs`.
Result: **2715 passed · 102 failed** (2026-04-21, after conformance-completeness signature/receiver/setter checks + move-tracker parity wins). stdlib/* stays clean (0 failures); all remaining failures live in non-stdlib trees. Note: `validation/cycles/protocol_direct_self_inheritance.ks` must be skipped to avoid a stack-overflow crash in the current WIP cycle analyzer (run with `--skip protocol_direct_self_inheritance`).

> **Agent instructions:** When you fix a failing test (or verify that an existing entry has become passing), move it to `test-errors-fixed.md`. Move the full bullet — the `[x]` marker, the failure mode, and any explanation — preserving its subsection heading for context. If a subsection's last remaining item is being moved, move the subsection heading and its explanatory prose with it. `[x]` entries must never sit in **# False Negatives** or **# Stdlib** — those lists are for still-failing `[ ]` items only. Do not modify a test's source to make it pass; if a test is genuinely invalid (wrong syntax, etc.), note that in the entry.

---

# False Negatives

Compiler fails to emit a diagnostic for code that should be rejected.

## Move / ownership / use-after-move checks not running

Borrow/move-checker not executing or not wired into bind→infer→validate pipeline.

> **lib1:** emitted by `kestrel-semantic-tree-binder/src/body_resolver/move_tracker.rs` + `diagnostics/move_tracking.rs`, `diagnostics/deinit.rs`, `diagnostics/copy_semantics.rs` (move-tracking runs inside body resolution, per-branch join state).

- [ ] `memory_model/deinit/deinit_statement_marks_variable_as_moved.ks` — **expected:** `moved`

## Protocol conformance not checked

`extend Foo: Proto { … }` doesn't verify method signatures, presence, or return types.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/conformance/` (surface check), `protocol_method/` (method signature / receiver-kind match), `parent_protocol_conformance/` (missing parent protocol), `protocol_field_conformance/` (setter/getter shape). Diagnostics in each analyzer's `diagnostics.rs`.

- [ ] `declarations/protocols/protocol_missing_method_from_inherited_protocol.ks` — **expected:** `does not implement method 'a'`
- [ ] `declarations/protocols/struct_missing_inherited_protocol_method.ks` — **expected:** `does not implement method 'draw'`
- [ ] `declarations/protocols/struct_with_method_wrong_return_type.ks` — **expected:** `method 'hash' has wrong return type`
- [ ] `declarations/protocols/diamond_inheritance_associated_type_conflict.ks` — **expected:** `conflicting associated type 'Element'`
- [ ] `declarations/extensions/no_transitive_conformance_when_chain_broken.ks` — **expected:** `does not satisfy constraint`
- [ ] `execution_graph/protocols/missing_parent_conformance_is_error.ks` — **expected:** `conforms to 'B' but not its parent protocol 'A'`
- [ ] `declarations/init_where_clauses/constraint_not_satisfied.ks` — **expected:** `Hashable`

## Overload resolution / ambiguity not diagnosed

Wrong-arity / wrong-label calls produce generic "wrong number of arguments" instead of the richer "no matching overload" the tests want; multiple ambiguity cases surface no error at all.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/type_inference/diagnostics.rs` ("no matching overload", "ambiguous") — overload scoring happens in `kestrel-semantic-type-inference` and the analyzer reports the verdict. `extension_conflict/` handles cross-extension ambiguity; `duplicate_callable/` catches duplicate signatures at declaration time.

- [ ] `declarations/protocol_method_linking/ambiguous_method_satisfies_multiple_protocols.ks` — **expected:** `ambiguous`
- [ ] `declarations/associated_types/ambiguous_associated_type_without_qualification.ks` — **expected:** `ambiguous associated type`
- [ ] `types/generics/constraint_enforcement/wrong_labels_on_constrained_call.ks` — **expected:** `wrong argument label` · **got:** `no member 'calculate' on type 'T'`

## Cloneable / Copyable / `not` negative-conformance rules

`not Copyable` + `Cloneable` conflict is not detected; `not` with non-builtin / method-bearing protocols is not rejected.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/disallowed_conformance/` (Cloneable + not-Copyable conflict, `not` applied to non-feature protocols), `cloneable_field/` (struct/enum payload needs Cloneable), `builtin_marker_protocol/` (Copyable marker-shape check, also the false-positive side).

- [ ] `memory_model/cloneable/cloneable_and_not_copyable_is_error.ks` — **expected:** `` cannot conform to `Cloneable` and opt out of `Copyable` ``
- [ ] `memory_model/cloneable/calling_generic_clone_with_non_cloneable_type_errors.ks` — **expected:** any error
- [ ] `memory_model/cloneable/enum_with_cloneable_payload_without_conformance_errors.ks` — **expected:** `Cloneable`
- [ ] `memory_model/cloneable/struct_with_cloneable_field_without_conformance_errors.ks` — **expected:** `Cloneable`
- [ ] `memory_model/negative_conformance/cloneable_and_not_copyable_is_conflicting.ks` — **expected:** `` cannot conform to `Cloneable` and opt out of `Copyable` ``
- [ ] `memory_model/negative_conformance/cloneable_and_not_copyable_reversed_order.ks` — **expected:** same
- [ ] `memory_model/negative_conformance/enum_cloneable_and_not_copyable_is_conflicting.ks` — **expected:** same
- [ ] `memory_model/negative_conformance/not_with_builtin_that_has_no_implicit_conformance.ks` — **expected:** `not a language feature protocol`
- [ ] `memory_model/negative_conformance/not_with_non_builtin_protocol.ks` — **expected:** `not a language feature protocol`
- [ ] `memory_model/negative_conformance/not_with_regular_protocol_that_has_methods.ks` — **expected:** `not a language feature protocol`

## Exhaustive-return analysis

Tests expect a specific "missing return on some paths" diagnostic; compiler instead emits the generic "expected i64 got ()" or E001 on a different line, which means the dedicated analysis isn't firing where it should.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/exhaustive_return/` — dedicated CFG-style pass. In lib2 the diagnostic leaks out of the type-check "expected T got ()" path instead of the return-path analysis.

- [ ] `validation/exhaustive_return/function_missing_return.ks` — **expected at line 8:** any error · **got at line 7:** E001 `does not return on all paths`
- [ ] `validation/exhaustive_return/if_else_chain_missing_final_else.ks` — **expected at line 12** · **got at line 7:** `expected i64 got ()`
- [ ] `validation/exhaustive_return/if_returns_else_falls_through.ks` — same pattern
- [ ] `validation/exhaustive_return/if_without_else_missing_return.ks` — same pattern
- [ ] `validation/exhaustive_return/loop_with_break_needs_return_after.ks` — **expected:** any error · **got:** none
- [ ] `validation/exhaustive_return/nested_if_inner_missing_else.ks` — **expected:** any error · **got:** `expected i64 got ()`
- [ ] `validation/exhaustive_return/statements_without_return.ks` — **expected at line 9** · **got at line 8:** E001
- [ ] `validation/exhaustive_return/while_loop_may_not_execute.ks` — **expected:** any error · **got:** none

## Visibility checks (public API surface uses private types)

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/visibility_consistency/` (public-surface-references-private diagnostics) + access-control checks in `kestrel-semantic-tree-binder/src/body_resolver/paths.rs` / `members.rs` for private-member access at call sites.

- [ ] `validation/misc/protocol_method_with_private_param_in_public_protocol_errors.ks` — **expected:** `parameter type in 'handle' is less visible`
- [ ] `validation/misc/public_field_with_private_type_errors.ks` — **expected:** `has type less visible than the field`
- [ ] `validation/misc/public_function_with_private_parameter_type_errors.ks` — **expected:** `parameter type in 'process' is less visible`
- [ ] `validation/misc/public_function_with_private_return_type_errors.ks` — **expected:** `return type of 'getSecret' is less visible`
- [ ] `validation/misc/public_type_alias_with_private_underlying_errors.ks` — **expected:** `aliased type in 'Exposed' is less visible`
- [ ] `validation/visibility/private_method_not_visible_outside_struct.ks` — **expected:** `is private and not accessible from this scope`
- [ ] `expressions/field_access/private_field_access_error.ks` — **expected:** `is private`

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

## Dictionary literal requires `Hashable` key — protocol-conformance diagnostic

Tests expect "does not conform to protocol" (Hashable); compiler emits generic type-mismatch (or nothing) instead.

> **lib1:** surfaced via the Hashable constraint on `Dictionary[K, V]`'s K param — `kestrel-semantic-type-inference` produces the conformance obligation and `kestrel-semantic-analyzers/src/analyzers/conformance/diagnostics.rs` emits "does not conform to protocol". Empty-dict "could not infer type" comes from `analyzers/type_inference/diagnostics.rs`.

- [ ] `expressions/dictionary_literals/empty_dict_without_context.ks` — **expected:** `could not infer type`
- [ ] `expressions/dictionary_literals/inconsistent_key_types.ks` — **expected:** `does not conform to protocol`
- [ ] `expressions/dictionary_literals/inconsistent_value_types.ks` — **expected:** `does not conform to protocol`
- [ ] `expressions/dictionary_literals/key_type_mismatch.ks` — **expected:** `does not conform to protocol`
- [ ] `expressions/dictionary_literals/value_type_mismatch.ks` — **expected:** `does not conform to protocol`

## `for x in nonIterable` — missing Iterable conformance diagnostic

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/for_loop_pattern/mod.rs` — checks the subject of `for` for Iterable conformance and reports if missing. Array-element-type mismatches for the inference variant fall through to the conformance analyzer.

- [ ] `patterns/for_loops/for_loop_over_non_iterable.ks` — **expected:** `Iterable`
- [ ] `inference/mod/infer_array_element_type_mismatch.ks` — **expected:** `does not conform to protocol`

## Syntax Sugar Errors

Desugarings (`for`, `try`, operators, etc.) fall through to raw member-lookup errors (`no member 'iter'`, `no member 'next'`) instead of emitting the intended protocol-conformance diagnostic.

- [ ] `patterns/for_loops/for_loop_over_non_iterator_without_iter_method.ks` — **expected:** `Iterable` · **got:** `no member 'iter' on type 'NotIterable'`, `does not conform to protocol: ? !: Iterator`, `no member 'next' on type '?'`
- [ ] `expressions/control_flow/try_on_non_tryable_type.ks` — **expected:** `tryExtract` · **got:** `.Err not found on ?`, `could not infer type` (desugar cascade)

## Match-expression diagnostics

> **lib1:** duplicate-binding-in-pattern + unknown-enum-case + wrong-arity (tuple/enum) emitted during pattern binding in `kestrel-semantic-tree-binder/src/body_resolver/patterns.rs` via `diagnostics/pattern.rs`. Float-literal-in-pattern and guard-must-be-Bool reported from the same path / `type_check` analyzer respectively.

- [ ] `expressions/match/errors/duplicate_binding_name.ks` — **expected:** `duplicate`
- [ ] `expressions/match/errors/float_literal_in_pattern.ks` — **expected:** `float`
- [ ] `expressions/match/errors/unknown_enum_case.ks` — **expected:** `Blue` (unknown case name)
- [ ] `expressions/match/errors/wrong_enum_arity.ks` — **expected:** any error
- [ ] `expressions/match/errors/wrong_tuple_arity.ks` — **expected:** `arity`
- [ ] `expressions/match/guards/guard_must_be_bool.ks` — **expected:** `Bool`
- [ ] `expressions/match/or_patterns/or_pattern_inconsistent_bindings_error.ks` — **expected:** `inconsistent` (bindings differ across `or`-pattern alternatives) · **got:** no error (plus a spurious `unreachable pattern [E306]` warning on the following arm)

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

## Empty array literal requires type annotation

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/type_inference/diagnostics.rs` — after inference finishes, unresolved infer-vars on array-literal element types produce the "could not infer type" diagnostic.

- [ ] `expressions/paths/empty_array_requires_type_annotation.ks` — **expected:** `could not infer type`

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

## Inherited associated type not resolved through parent protocol

`protocol Child: Base` with `type Element` declared on `Base`: resolving `Self.Element` / `Item` from inside `Child` (or a conforming struct) fails to walk the parent chain. Shows up as spurious "no associated type" or "wrong return type" diagnostics when the protocol requirement's return type is an inherited associated type.

- [ ] `declarations/associated_types/struct_conforming_to_child_provides_associated_type.ks` — **expected:** no errors · **got:** E458 `method 'prev' has wrong return type for protocol 'BidirectionalIterator'` at line 15 (impl returns `lang.i64`, matches `Item = lang.i64` but protocol-method return `Item` not substituted through the struct's associated-type binding when the requirement lives on a parent protocol)
- [ ] `declarations/extensions/protocol_extension_uses_inherited_associated_type.ks` — **expected:** no errors · **got:** `no associated type 'Element': Self.Element no assoc type` (lines 9, 12, 12) + `could not infer type` (line 13) — extension body can't project `Self.Element` from the parent `Base` through `Child`

---

# Stdlib

Run: `file_tests --test-threads=1` (full suite, stdlib/* subset) on `feature/incremental-hecs` (2026-04-21, fifth run).
Result: **203 passed · 0 failed** (full-suite extract: all stdlib/* tests pass). Fixed vs. fourth run: the remaining 8 entries in the stdlib Type-inference/bind-errors bucket cleared (`stdlib/array/init_count_generator.ks`, `stdlib/iterator/zip_chain_enumerate.ks` among them). stdlib has no tracked failures.

Previously resolved categories:
- **E205 `cannot pass temporary value to 'mutating' parameter`** — fully resolved 2026-04-20 (access-mode analyzer receiver/arg split + stdlib `mutating` → `consuming` flip). All former E205 tests reclassified below by their remaining failure.
- **Type parameter not in scope** — fixed 2026-04-20 (`WorldResolver::where_clauses` context fix). 3 stdlib tests now pass (`append_from_iterable`, `dictionary_merge_from_pairs`, `set_insert_contents_of`); 4 others moved to derived-protocol bucket.
- **Monomorphization witness gaps** — fixed 2026-04-20 via new `ProtocolMembers` query in `kestrel-name-res` that unifies the protocol-child + extension + parent-protocol walk. Witness generation and name-resolution consumers now call one query instead of reassembling the walk. 4 tests pass; 1 regressed to a separate pre-existing overload-collision bug; 20 others reclassified by new failure mode.
- **Witness-instantiation collapse** — fixed 2026-04-20. `ConformingProtocols` deduped by protocol entity so `Int64: Convertible[Int8], [Int16], [Int32], ...` collapsed into a single `Convertible` witness bound to the first `init(from:)` overload — every `Int64(from: x)` silently truncated x to 8 bits. Fix: new `ConformingProtocolInstantiations` query preserves per-conformance type args; `witness_lower.rs` emits one witness per `(protocol, type_args)` with parameter-type init disambiguation; codegen's `find_witness_with_method` filters by `protocol_type_args`. Net: −23 stdlib failures (integer conversions, parse, byte-endian, bitwidth ops, float conversions).
- **Codegen symbol not found: `Array.init`** — the `collect()` monomorphization miss is resolved. Nearly all former entries moved to Cranelift verifier errors (compile/link phase) or Runtime exit-code failures (runs but asserts fail); a couple now hit earlier MIR/inference errors. Only `try_fold_adapter` still links against an undeclared symbol, for a different monomorphization gap (`tryFold`).
- **Cranelift verifier `i64`/`i8` signature mismatch** — resolved 2026-04-21 (likely by the `self_item_leaked_to_mir` fix + surrounding monomorphization work). All 9 former entries (7 `call_indirect` arg-2 mismatches across `MapIterator`/`FilterMapIterator`/`InspectIterator`/`IntersperseIterator`/`TakeWhileIterator`, plus 2 `load.i64` base-pointer mismatches in `FlattenIterator`/`IntersperseWithIterator`) now compile and link cleanly. Reclassified into the Runtime exit-code bucket, then resolved in the fourth 2026-04-21 run.
- **Iterator-adapter runtime exit-code failures** — resolved 2026-04-21 (fourth run). 9 tests (`filter_map_flatten`, `flatten_iterator`, `fuse_and_cycle`, `inspect_adapter`, `intersperse_adapter`, `intersperse_with_adapter`, `map_filter_collect`, `peekable_adapter`, `take_skip_methods`) all pass. Mix of SIGSEGVs (Optional<I.Item> layout for nested adapters, generic-I discriminant handling) and assertion-exit failures resolved together.

## Runtime exit-code failures (compile OK, assert/behavior wrong)

Program compiles and links but exits non-zero — asserts failing or behavior diverging from expectation.

_(none currently tracked — see `test-errors-fixed.md` for resolved entries)_

