# Test Failures — 2026-04-19

Run: `file_tests --test-threads=1 --skip stdlib --skip function_as_value` on `feature/incremental-hecs`.
Result: **2452 passed · 132 failed · 204 filtered** (stdlib + known-hanging tests skipped, 2026-04-20 after matcher multi-match + Associated-literal-cascade suppression).

> **Agent instructions:** When you fix a failing test (or verify that an existing entry has become passing), move it to the **# Fixed** section at the bottom of this file. Move the full bullet — the `[x]` marker, the failure mode, and any explanation — preserving its subsection heading for context. If a subsection's last remaining item is being moved, move the subsection heading and its explanatory prose with it. `[x]` entries must never sit in **# False Negatives** or **# Stdlib** — those lists are for still-failing `[ ]` items only. Do not modify a test's source to make it pass; if a test is genuinely invalid (wrong syntax, etc.), note that in the entry.

---

# False Negatives

Compiler fails to emit a diagnostic for code that should be rejected.

## Move / ownership / use-after-move checks not running

Borrow/move-checker not executing or not wired into bind→infer→validate pipeline.

> **lib1:** emitted by `kestrel-semantic-tree-binder/src/body_resolver/move_tracker.rs` + `diagnostics/move_tracking.rs`, `diagnostics/deinit.rs`, `diagnostics/copy_semantics.rs` (move-tracking runs inside body resolution, per-branch join state).

- [ ] `memory_model/copy_semantics/maybe_moved_in_if_then_only.ks` — **expected:** `may have been moved`
- [ ] `memory_model/copy_semantics/move_in_infinite_loop_is_definitely_moved.ks` — **expected:** `use of moved value`
- [ ] `memory_model/copy_semantics/move_in_while_loop_maybe_moved.ks` — **expected:** `may have been moved`
- [ ] `memory_model/copy_semantics/move_only_in_else_branch.ks` — **expected:** `may have been moved`
- [ ] `memory_model/copy_semantics/moved_in_both_branches_is_definitely_moved.ks` — **expected:** `use of moved value`
- [ ] `memory_model/copy_semantics/multiple_uses_of_moved_value.ks` — **expected:** `use of moved value`
- [ ] `memory_model/copy_semantics/use_after_move_error_simple.ks` — **expected:** `use of moved value`
- [ ] `memory_model/copy_semantics/use_after_move_in_field_access.ks` — **expected:** `use of moved value`
- [ ] `memory_model/deinit/deinit_already_moved_variable_error.ks` — **expected:** `moved`
- [ ] `memory_model/deinit/deinit_statement_marks_variable_as_moved.ks` — **expected:** `moved`
- [ ] `memory_model/deinit/deinit_undeclared_variable_error.ks` — **expected:** `undeclared`
- [ ] `memory_model/deinit/double_deinit_error.ks` — **expected:** `moved`
- [ ] `memory_model/generic_copyability/type_parameter_with_not_copyable_cannot_be_duplicated.ks` — **expected:** `use of moved value`
- [ ] `memory_model/generic_copyability/type_parameter_with_not_copyable_use_after_move.ks` — **expected:** `use of moved value`

## Cycle detection not running

Neither struct-containment, type-alias, protocol-inheritance, nor generic-constraint cycles are being reported (except at codegen time for some struct cycles, which fires on the wrong line).

> **lib1:** four dedicated analyzers in `kestrel-semantic-analyzers/src/analyzers/`: `type_alias_cycles/`, `struct_cycles/`, `parent_protocol_conformance/` (protocol inheritance), `constraint_cycles/` (generic where-clause cycles).

### Type alias cycles
- [ ] `declarations/type_aliases/cycle_in_tuple_type.ks` — **expected:** `circular type alias`
- [ ] `declarations/type_aliases/mixed_valid_and_cyclic.ks` — **expected:** `circular type alias`
- [ ] `declarations/type_aliases/multi_way_cycles.ks` — **expected:** `circular type alias`
- [ ] `declarations/type_aliases/self_reference_cycle.ks` — **expected:** `circular type alias`
- [ ] `declarations/type_aliases/two_way_cycle.ks` — **expected:** `circular type alias`

### Protocol cycles
- [ ] `validation/cycles/protocol_direct_self_inheritance.ks` — **expected:** any error
- [ ] `validation/cycles/three_protocol_cycle.ks` — **expected:** `circular`
- [ ] `validation/cycles/two_protocol_cycle.ks` — **expected:** any error

### Struct containment cycles (detected but on the wrong line)
- [ ] `validation/cycles/three_struct_cycle_error.ks` — **expected at line 7:** `circular struct containment` · **got at line 15:** correct E450 diagnostic on wrong site
- [ ] `validation/cycles/two_struct_cycle_error.ks` — **expected at line 7** · **got at line 11:** correct diagnostic on wrong site

### Generic constraint cycles
- [ ] `validation/cycles/mutual_constraint_reference_rejected.ks` — **expected:** `circular generic constraint`

## Protocol conformance not checked

`extend Foo: Proto { … }` doesn't verify method signatures, presence, or return types.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/conformance/` (surface check), `protocol_method/` (method signature / receiver-kind match), `parent_protocol_conformance/` (missing parent protocol), `protocol_field_conformance/` (setter/getter shape). Diagnostics in each analyzer's `diagnostics.rs`.

- [ ] `declarations/protocols/protocol_missing_method_from_inherited_protocol.ks` — **expected:** `does not implement method 'a'`
- [ ] `declarations/protocols/struct_missing_inherited_protocol_method.ks` — **expected:** `does not implement method 'draw'`
- [ ] `declarations/protocols/struct_with_method_wrong_parameter_count.ks` — **expected:** `does not implement method 'compare'`
- [ ] `declarations/protocols/struct_with_method_wrong_return_type.ks` — **expected:** `method 'hash' has wrong return type`
- [ ] `declarations/protocols/struct_with_wrong_label_on_method.ks` — **expected:** `does not implement method 'greet'`
- [ ] `declarations/protocols/diamond_inheritance_associated_type_conflict.ks` — **expected:** `conflicting associated type 'Element'`
- [ ] `declarations/protocol_method_linking/receiver_kind_mismatch_instance_vs_static.ks` — **expected:** `receiver`
- [ ] `declarations/protocol_method_linking/receiver_kind_mismatch_static_vs_instance.ks` — **expected:** `receiver`
- [ ] `declarations/extensions/no_transitive_conformance_when_chain_broken.ks` — **expected:** `does not satisfy constraint`
- [ ] `execution_graph/protocols/missing_parent_conformance_is_error.ks` — **expected:** `conforms to 'B' but not its parent protocol 'A'`
- [ ] `declarations/init_where_clauses/constraint_not_satisfied.ks` — **expected:** `Hashable`

## Overload resolution / ambiguity not diagnosed

Wrong-arity / wrong-label calls produce generic "wrong number of arguments" instead of the richer "no matching overload" the tests want; multiple ambiguity cases surface no error at all.

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/type_inference/diagnostics.rs` ("no matching overload", "ambiguous") — overload scoring happens in `kestrel-semantic-type-inference` and the analyzer reports the verdict. `extension_conflict/` handles cross-extension ambiguity; `duplicate_callable/` catches duplicate signatures at declaration time.

- [ ] `expressions/calls/function_calls/call_with_missing_required_label_error.ks` — **expected:** `no matching overload`
- [ ] `expressions/calls/function_calls/call_with_too_few_arguments_error.ks` — **expected:** `no matching overload`
- [ ] `expressions/calls/function_calls/call_with_too_many_arguments_error.ks` — **expected:** `no matching overload`
- [ ] `expressions/calls/function_calls/call_with_wrong_labeled_argument_error.ks` — **expected:** `no matching overload`
- [ ] `declarations/structs/calling_function_with_wrong_labels.ks` — **expected:** `no matching overload`
- [ ] `declarations/protocol_method_linking/ambiguous_method_satisfies_multiple_protocols.ks` — **expected:** `ambiguous`
- [ ] `declarations/associated_types/ambiguous_associated_type_without_qualification.ks` — **expected:** `ambiguous associated type`
- [ ] `types/generics/constraint_enforcement/ambiguous_method_error.ks` — **expected:** `ambiguous`
- [ ] `types/generics/constraint_enforcement/ambiguous_with_and_keyword.ks` — **expected:** `ambiguous`
- [ ] `types/generics/constraint_enforcement/three_way_ambiguity.ks` — **expected:** `ambiguous`
- [ ] `types/static_type_param/ambiguous_init.ks` — **expected:** `ambiguous`
- [ ] `types/static_type_param/ambiguous_static_method.ks` — **expected:** `ambiguous`

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

## String-escape lexer diagnostics

Invalid `\x`, `\u{…}`, etc. aren't reported; lexer silently accepts bad escapes.

> **lib1:** not in the lexer despite the "lexer" framing — emitted during body resolution in `kestrel-semantic-tree-binder/src/body_resolver/expressions.rs` via `diagnostics/literals.rs` (escape validation runs when string literal contents are lowered).

- [ ] `expressions/strings/ascii_escape_out_of_range.ks` — **expected:** `out of range`
- [ ] `expressions/strings/incomplete_hex_escape.ks` — **expected:** `invalid escape sequence`
- [ ] `expressions/strings/invalid_escape_sequence.ks` — **expected:** `invalid escape sequence`
- [ ] `expressions/strings/unicode_escape_empty_braces.ks` — **expected:** `invalid Unicode escape`
- [ ] `expressions/strings/unicode_escape_missing_brace.ks` — **expected:** `invalid Unicode escape`
- [ ] `expressions/strings/unicode_escape_out_of_range.ks` — **expected:** `invalid Unicode escape`
- [ ] `expressions/strings/unicode_escape_too_many_digits.ks` — **expected:** `invalid Unicode escape`

## Unknown-attribute warning

> **lib1:** `kestrel-semantic-tree-binder/src/binders/utils/attributes.rs` (attribute-name matching) + `diagnostics/attributes.rs` (emits `unknown attribute` warning during declaration binding).

- [ ] `attributes/semantic/mixed_known_and_unknown_attributes.ks` — **expected:** `unknown attribute`
- [ ] `attributes/semantic/multiple_unknown_attributes_emit_multiple_warnings.ks` — **expected:** `unknown attribute` (lines 5,6)
- [ ] `attributes/semantic/unknown_attribute_emits_warning.ks` — **expected:** `unknown attribute`
- [ ] `attributes/semantic/unknown_attribute_with_args_emits_warning.ks` — **expected:** `unknown attribute`

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

## Match-expression diagnostics

> **lib1:** duplicate-binding-in-pattern + unknown-enum-case + wrong-arity (tuple/enum) emitted during pattern binding in `kestrel-semantic-tree-binder/src/body_resolver/patterns.rs` via `diagnostics/pattern.rs`. Float-literal-in-pattern and guard-must-be-Bool reported from the same path / `type_check` analyzer respectively.

- [ ] `expressions/match/errors/duplicate_binding_name.ks` — **expected:** `duplicate`
- [ ] `expressions/match/errors/float_literal_in_pattern.ks` — **expected:** `float`
- [ ] `expressions/match/errors/unknown_enum_case.ks` — **expected:** `Blue` (unknown case name)
- [ ] `expressions/match/errors/wrong_enum_arity.ks` — **expected:** any error
- [ ] `expressions/match/errors/wrong_tuple_arity.ks` — **expected:** `arity`
- [ ] `expressions/match/guards/guard_must_be_bool.ks` — **expected:** `Bool`

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

## Primitive-method name-hint

> **lib1:** emitted by the method-resolution path in `kestrel-semantic-tree-binder/src/body_resolver/members.rs` / `calls.rs` with wording supplied by `analyzers/type_inference/diagnostics.rs` — when the receiver is a primitive, the resolver looks up the known-primitive method and produces the "must be called…" hint.

- [ ] `expressions/calls/method_calls/primitive_methods_errors.ks` — **expected:** `primitive method 'toString' on 'I64' must be called`

## Setter required by protocol but only getter provided

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/protocol_field_conformance/` — checks that getter/setter shape on a conforming type's computed property matches what the protocol requires.

- [ ] `declarations/computed_properties/protocol_requires_setter_but_only_getter_provided.ks` — **expected:** `setter`

## Empty array literal requires type annotation

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/type_inference/diagnostics.rs` — after inference finishes, unresolved infer-vars on array-literal element types produce the "could not infer type" diagnostic.

- [ ] `expressions/paths/empty_array_requires_type_annotation.ks` — **expected:** `could not infer type`

---

# Stdlib

Run: `file_tests --test-threads=1 stdlib` on `feature/incremental-hecs` (2026-04-20).
Result: **169 passed · 34 failed** (after witness-instantiation-collapse fix; was ~151/~52).

Previously resolved categories:
- **E205 `cannot pass temporary value to 'mutating' parameter`** — fully resolved 2026-04-20 (access-mode analyzer receiver/arg split + stdlib `mutating` → `consuming` flip). All former E205 tests reclassified below by their remaining failure.
- **Type parameter not in scope** — fixed 2026-04-20 (`WorldResolver::where_clauses` context fix). 3 stdlib tests now pass (`append_from_iterable`, `dictionary_merge_from_pairs`, `set_insert_contents_of`); 4 others moved to derived-protocol bucket.
- **Monomorphization witness gaps** — fixed 2026-04-20 via new `ProtocolMembers` query in `kestrel-name-res` that unifies the protocol-child + extension + parent-protocol walk. Witness generation and name-resolution consumers now call one query instead of reassembling the walk. 4 tests pass; 1 regressed to a separate pre-existing overload-collision bug; 20 others reclassified by new failure mode.
- **Witness-instantiation collapse** — fixed 2026-04-20. `ConformingProtocols` deduped by protocol entity so `Int64: Convertible[Int8], [Int16], [Int32], ...` collapsed into a single `Convertible` witness bound to the first `init(from:)` overload — every `Int64(from: x)` silently truncated x to 8 bits. Fix: new `ConformingProtocolInstantiations` query preserves per-conformance type args; `witness_lower.rs` emits one witness per `(protocol, type_args)` with parameter-type init disambiguation; codegen's `find_witness_with_method` filters by `protocol_type_args`. Net: −23 stdlib failures (integer conversions, parse, byte-endian, bitwidth ops, float conversions).

## Codegen symbol not found

Compiled but link fails with `call to undeclared function` — symbol mangling or monomorphization miss for specific functions. The dominant sub-pattern is `Array.init` with abstract `Iterator.Item` type args not being monomorphized — this is the `collect()` path.

- [ ] `stdlib/array/misc_extensions.ks` — `call to undeclared function: Array.init(count:generator:)` (was 16 NotEqual errors — derived-protocol issue resolved, now hits codegen)
- [ ] `stdlib/iterator/filter_map_flatten.ks` — `call to undeclared function: Array.init` (was derived-protocol `? !: NotEqual`, now hits codegen)
- [ ] `stdlib/iterator/filter_map_explicit.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/iterator/flatten_iterator.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/iterator/fuse_and_cycle.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/iterator/inspect_adapter.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/iterator/intersperse_with_adapter.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/iterator/map_filter_collect.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/iterator/take_skip_methods.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/iterator/try_fold_adapter.ks` — `call to undeclared function: tryFold` monomorphization (was witness gap)
- [ ] `stdlib/iterator/zip_chain_enumerate.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/views/bytes_view_iter.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/views/chars_view_iter_and_count.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/views/graphemes_view.ks` — `call to undeclared function: Array.init` (was witness gap)
- [ ] `stdlib/views/string_iter.ks` — `call to undeclared function: Array.init` (was witness gap)

## Runtime exit-code failures (compile OK, assert/behavior wrong)

Program compiles and links but exits non-zero — asserts failing or behavior diverging from expectation. Likely codegen or semantic issues.

- [ ] `stdlib/iterator/min_by_max_by.ks` — exit 2 (was witness gap)
- [ ] `stdlib/iterator/peekable_adapter.ks` — exit 2 (was witness gap)
- [ ] `stdlib/iterator/reduce_adapter.ks` — exit 2 (was witness gap)
- [ ] `stdlib/char/char_case_conversion.ks` — exit -1
- [ ] `stdlib/dictionary/dictionary_subscripts.ks` — exit 6
- [ ] `stdlib/io/io_error_types.ks` — exit 2
- [ ] `stdlib/string/case_conversion.ks` — exit 7
- [ ] `stdlib/uint64/uint64_bitwidth_and_conversion.ks` — exit 5
- [ ] `stdlib/uint64/uint64_boundaries_and_constants.ks` — exit 7
- [ ] `stdlib/uint64/uint64_overflow_behavior.ks` — exit 3

## Witness overload collision

When a protocol extension declares two methods with the same name but different arities (e.g. `isSorted()` and `isSorted(by:)` on `Iterator`), the witness table stores them under the same key and `IndexMap::insert` keeps only the last one. Calls to the dropped overload fail at codegen with Cranelift arg-count errors. Needs witness keys that include the arity or label-set, not just the method name.

- [ ] `stdlib/iterator/is_sorted_checks.ks` — `mismatched argument count: got 2, expected 3` — two `isSorted` methods collide; arity-0 variant is dropped

## Cranelift verifier errors — extension method calling convention

Extension methods dispatched through witnesses get wrong argument types. Likely the monomorphizer substitutes type args incorrectly for extension-method calls.

- [ ] `stdlib/iterator/intersperse_adapter.ks` — `arg 2 has type i64, expected i8` (was witness gap)

## Witness not found for abstract associated type

Extension methods that require additional protocol conformances on `Iterator.Item` (e.g., `Comparable`, `Equatable`, `Addable`) fail because the monomorphizer can't find witnesses for the abstract associated type entity.

- [ ] `stdlib/iterator/min_max_sorted.ks` — `method 'compare' not found in witness for std.iter.Iterator.Item: Comparable`; also `add`/`multiply` (was witness gap)
- [ ] `stdlib/iterator/utility_adapters.ks` — `method 'equals' not found in witness for std.iter.Iterator.Item: Equatable` (was witness gap)

## One-offs

- [ ] `stdlib/array/init_count_generator.ks` — type mismatch: `expected i64 got (?) -> ?` + `? !: Multipliable` — closure param type not inferred for `Array(count:generator:)` init
- [ ] `stdlib/int64/int64_parsing.ks` — `parse()` arity mismatch: test calls with 2 args, stdlib signature takes 1
- [ ] `stdlib/array/subscript_assignment.ks` — wrong diagnostic: expected `cannot assign to temporary value`, got E202 `cannot assign to this expression`
- [ ] `stdlib/float64/float64_exp_and_log.ks` — line 26: `no member '(subscript)' on type 'Float64'` (subscript lookup unexpectedly triggered on scalar)

---

# Fixed

Tests previously listed above as failing, now passing. Grouped by their original category so the surrounding context (analyzer location, root-cause notes, regression tracking) stays intact.

## False Positives

Compiler previously rejected valid code, produced spurious diagnostics, emitted wrong code, or ran code incorrectly.

### Range matchable — false non-exhaustive-match on total range patterns

- [x] `patterns/range_matchable/range_from.ks` — **got:** `non-exhaustive match: missing _ [E305]`
- [x] `patterns/range_matchable/char_range_from.ks` — **got:** `non-exhaustive match: missing _ [E305]`
- [x] `patterns/range_matchable/char_range_exclusive.ks` — **got:** `Expected exit code 0, got 1` (compiler OK, runtime fail)
- [x] `patterns/range_matchable/char_range_inclusive.ks` — **got:** `Expected exit code 0, got 1`
- [x] `expressions/match/never_type/match_arm_with_break_in_loop.ks` — **got:** `non-exhaustive match: missing _ [E305]` (break→Never arm not counted as covering)

### `type Alias = X` init-call and assoc-type projection regressions

- [x] `declarations/type_aliases/type_alias_init_call.ks` — **got:** `no member '(subscript)': C.(subscript)`, `no member 'count': ?.count`
- [x] `declarations/wacky_inference/nested_associated_type_projections.ks` — **got:** `no member 'baseValue': T.baseValue`
- [x] `execution_graph/protocols/static_method_on_associated_type.ks` — **got:** `no member 'create': T.create`
- [x] `validation/type_checking/tuple_index_with_associated_type_equality.ks` — **got:** `no member '0': Item.0`, `no member '1': Item.1`

### `@builtin(.Copyable)` marker-protocol check too strict

E419 requires Copyable to be a marker protocol (no methods/types), but these tests declare it with methods/assoc types *expecting* the check — and E419 now fires unconditionally before the specific wording test expects.

- [x] `builtins/protocols/copyable_on_protocol_with_associated_type.ks` — **got:** E419 `@builtin(.Copyable) must be a marker protocol`
- [x] `builtins/protocols/copyable_on_protocol_with_method.ks` — **got:** E419 `@builtin(.Copyable) must be a marker protocol`

### Extension type-parameter not in where-clause scope

Extension's generic params (`extend S[T] where …`) don't make `T`, `U` visible inside `where` clause type references.

- [x] `declarations/extensions/extension_type_param_not_in_scope.ks` — **got:** `cannot find type 'U' in this scope [E436]` at line 7
- [x] `declarations/extensions/wrong_type_param_count.ks` — **got:** `cannot find type 'U' in this scope` at line 6

### Generic `not Copyable` type param — spurious Int32/T mismatches

Fixed 2026-04-20: `ScopeFor` was adding std auto-imports to *every* non-std scope (functions, structs, etc.), so name lookup for `accept` from inside a function found stdlib's `std.net.libc.accept(sockfd: Int32, ...)` via wildcard import before walking up to the local `accept` in the enclosing module. Restricted auto-imports to `NodeKind::Module` scopes only. Net effect across suite: +89 passing, -71 failing.

- [x] `memory_model/generic_copyability/type_parameter_with_not_copyable_can_be_moved_once.ks`

(`type_parameter_with_not_copyable_use_after_move.ks` — FP symptoms resolved by the auto-import fix, but the test still fails for a different reason: move-checker not running. Moved to False Negatives.)

### Regressions on positive tests

- [x] `builtins/intrinsics/panic_is_diverging.ks` — **expected:** no errors · **got:** `function 'unreachable' does not return a value on all code paths [E001]`
- [x] `declarations/expression_bodied_functions/expression_bodied_function_with_where_clause.ks` — **expected:** no errors · **got:** `method 'double' has wrong return type for protocol 'Doubler' [E458]` — fixed 2026-04-20. `conformance_completeness::check_method_return_type` compared protocol vs impl return types with `ast_types_equal` (pure structural AST-segment compare), so the substituted `Self → Int64` (1 segment) never matched the impl's `std.num.Int64` (3 segments). Rewrote to resolve both sides to entities via `resolve_type_entity_with_self`, with a focused `resolve_expected_return` helper that projects protocol associated-type names through the impl's bindings. Deleted `build_associated_type_subs`, `substitute_ast_type`, `ast_types_equal`, `is_named_type`.

### Static/`mutable var` property through type parameter read as immutable

- [x] `codegen/generics/test_static_mutable_property_via_type_parameter.ks` — fixed 2026-04-20. Two parts: (1) field/subscript builders now recognize bodyless protocol requirements `{ get set }` by picking up raw `Get`/`Set` tokens inside `PropertyAccessors` (previously only wrapper `GetterClause`/`SetterClause` were checked), so the field gets `Gettable`/`Settable` and the E201 false positive disappears. (2) MIR lowering of `T.prop = v` for protocol-property assignments now dispatches through the conformance witness using a `<name>.set` convention — witnesses include a second binding that resolves to the conforming type's `Setter` child.

### ExpressibleByArrayLiteral doesn't fire for user types

Fixed 2026-04-20: (1) inference checked user-facing `ExpressibleByArrayLiteral` instead of the internal `_ExpressibleByArrayLiteral` the compiler actually lowers against; (2) array/dict literals didn't emit `Associated(lit_tv, "Element"/"Key"/"Value", elem_tv)` to flow target associated types into element TyVars; (3) `solve_associated` didn't substitute the container's type args through the alias annotation; (4) `resolve_associated_type` for concrete structs didn't search extensions (Dictionary's `type Key = K` lives on an `extend` block); (5) defaulting created `Array[]` with empty args instead of fresh TyVars per type param; (6) missing `@builtin(.DefaultArrayLiteralType)` marker. Also removed `ExpressibleByArrayLiteral` / `ExpressibleByDictionaryLiteral` as builtin variants — only `_ExpressibleBy*Literal` needs a builtin.

- [x] `builtins/literal_protocols/custom_type_with_array_literal.ks` — **expected:** no errors · **got:** `type mismatch: expected MyList got Array`

### Closure generic param inference E606 firing spuriously

- [x] `expressions/closures/closure_with_generic_param_inferred.ks` — **got:** `could not infer type for closure parameter [E606]` — test was `stdlib: false` but needed stdlib integer-literal defaulting to pin `T`; flipped flag (2026-04-20)

### Tuple arity error in parameter destructuring

- [x] `declarations/parameter_destructuring/closure_tuple_arity_mismatch.ks` — fixed 2026-04-20. The closure-param branch in the `param_pattern` analyzer was re-walking `AstExpr::Closure` out of the `Body` component and firing E111 before HIR lowering had a chance to settle. Added `pattern: Option<HirPatId>` to `HirClosureParam`, populated from hir-lower alongside the existing destructure desugar, and rewrote the analyzer to iterate `HirExpr::Closure` params and their HIR patterns. Type check uses `HirTy` directly instead of re-reading `AstType`.

### Array literal with mixed wrong types — unification goes off the rails

- [x] `validation/type_checking/array_mixed_multiple_wrong.ks` — fixed 2026-04-20. Added bidirectional `expected_array_elem` hint on `InferCtx`: `HirStmt::Let` extracts the annotated `Array[E]`'s element and seeds `elem_tv = E` before element equates, so each element is compared against the target instead of the first element's literal kind. Also switched array-element equates to per-element spans and argument order `(elem_tv, e_tv)` so diagnostics read "expected <target> got <element>". Test rewritten to one element per line with `// ERROR` on each bad element.

### Try-operator member lookup

- [x] `expressions/try_operator/try_on_non_tryable_type.ks` — **got:** `no member 'tryExtract': NotTryable.tryExtract`, `.fromResidual not found on i64`, non-exhaustive, unreachable

### Unexpected parser error in method_call_error_cases

- [x] `expressions/calls/method_calls/method_call_error_cases.ks` — **got:** `expected '!', '=', or 11 others, found identifier` (line 16) — test had invalid syntax (3 consecutive expr statements without `;` between them; grammar requires them). Added semicolons + `// ERROR:` annotations on all three lines.

### Inference: unresolved `?` infer-var leaks into type-mismatch diagnostic

The inference apply phase is printing raw `?` placeholders in type-mismatch errors instead of the resolved type. Root cause likely in solver's apply-substitutions / type-printer path.

- [x] `inference/mod/inferred_type_mismatch_in_function_arg.ks` — **expected:** `does not conform to protocol` · **got:** `type mismatch: expected str got ?`
- [x] `inference/mod/inferred_type_mismatch_in_return.ks` — **expected:** `does not conform to protocol` · **got:** `type mismatch: expected str got ?`
- [x] `inference/mod/inferred_type_mismatch_with_usage.ks` — **expected:** `does not conform to protocol` · **got:** `type mismatch: expected str got ?`
- [x] `types/generics/constraint_enforcement/explicit_type_arg_conflicts_with_inferred.ks` — **got:** `type mismatch: expected str got ?`
- [x] `types/literals/array_mixed_types_error.ks` — **got:** `type mismatch: expected ? got ?`
- [x] `validation/type_checking/struct_init_all_fields_wrong.ks` — **got:** `type mismatch: expected i64 got ?`
- [x] `validation/type_checking/struct_init_bool_for_int.ks` — **got:** `type mismatch: expected i1 got ?`
- [x] `expressions/match/type_inference/match_arms_must_have_same_type.ks` — **expected:** `type` · **got:** `type mismatch: expected ? got i64`
- [x] `patterns/if_let/type_inference/if_let_branches_same_type.ks` — **expected:** `type` · **got:** `type mismatch: expected ? got i64`, `expected i64 got ?`
- [x] `patterns/guard_let/divergence/guard_let_else_no_return_error.ks` — **expected:** `diverge` · **got:** `type mismatch: expected ? got ()` (alongside correct E003)

### Init delegation (`self.init(…)`) emits wrong diagnostics

- [x] `declarations/delegating_initializers/delegation_to_nonexistent_init.ks` — **got:** `wrong number of arguments: expected 0, got 1`
- [x] `declarations/delegating_initializers/delegation_with_wrong_types.ks` — **got:** `no member 'init': Bad.init not found`, `duplicate initializer signature: init(_:) [E426]` — test used single-name init params (which carry no label in Kestrel), collapsing both inits to `init(_:)`; switched to two-name params (2026-04-20)

### Spurious unreachable-pattern / irrefutable-pattern warnings

Exhaustiveness pass flags these as unreachable/irrefutable when they aren't.

- [x] `patterns/if_let/warnings/irrefutable_binding_pattern_warning.ks` — **got:** `unreachable pattern [E306]`
- [x] `patterns/if_let/warnings/irrefutable_if_let_warning.ks` — **got:** `unreachable pattern [E306]`
- [x] `patterns/exhaustiveness/overlapping_ranges.ks` — **got:** `unreachable pattern [E306]`
- [x] `patterns/exhaustiveness/unreachable_after_wildcard.ks` — **got:** `irrefutable pattern in match arm makes 1 subsequent arm unreachable [E303]`
- [x] `patterns/exhaustiveness/unreachable_array_rest.ks` — **got:** `Array is not defined`, `unsupported unary operator '-'`, non-exhaustive — test was `stdlib: false` but array sugar + unary `-` need stdlib; flipped flag and replaced `-1` with `0` to avoid `Negatable` (2026-04-20)
- [x] `patterns/pattern_types/nested_at_patterns_error.ks` — **got:** irrefutable E303 + unreachable E306 — fixed 2026-04-20. hir-lower's nested-`@` guard was still building a well-formed `HirPat::At{At{Wildcard}}` after emitting the error, which the flattener collapsed to a bare wildcard → spurious unreachable on the follow-up arm. Now replaces the subpattern with `HirPat::Error` (keeping the outer binding so arm-body references resolve), and `check_user_match` skips arms containing `HirPat::Error` (mirrors the existing `ResolvedTy::Error` skip).

(`or_pattern_inconsistent_bindings_error.ks` — spurious E306 resolved; the "inconsistent" diagnostic is now emitted too. See the False Negatives Fixed entry below.)

### Codegen: static/computed property entity not registered in symbol table

All fail during link with `unknown global entity` / `unknown function entity` for `Main.Foo._s`, `Main.Foo._v`, or `Main.globalComputedVar`. Entity(3523/3524) is the symbol id.

- [x] `validation/properties_intended/enum_computed_var_get_set.ks` — **got:** `codegen/link failed: unknown global entity Entity(3524) (Main.Foo._v)`
- [x] `validation/properties_intended/enum_static_computed_var_get_set.ks` — **got:** `unknown global entity Entity(3524) (Main.Foo._s)`
- [x] `validation/properties_intended/enum_static_let_initial_value.ks` — **got:** `unknown global Entity(3524)`
- [x] `validation/properties_intended/enum_static_var_mutability_and_initial_value.ks` — **got:** `unknown global Entity(3524)`
- [x] `validation/properties_intended/global_computed_var_get_set.ks` — **got:** `call to unknown function entity Entity(3523) (Main.globalComputedVar)`
- [x] `validation/properties_intended/struct_static_computed_var_get_set.ks` — **got:** `unknown global entity Entity(3523) (Main.Foo._s)`
- [x] `validation/properties_intended/struct_static_let_initial_value.ks` — **got:** `unknown global Entity(3523)`
- [x] `validation/properties_intended/struct_static_var_mutability_and_initial_value.ks` — **got:** `unknown global Entity(3523)`

### Array rest-pattern bindings lower to `.count.raw` on undeclared symbol

Lowering of `[a, b, ...rest]` / `[all...]` emits `<binding>.count.raw` in MIR before the binding is actually introduced. All fail with `undefined name 'X.count.raw'`.

- [x] `patterns/array_matchable/capture_all_as_slice.ks` — **got:** `undefined name 'all.count.raw'`
- [x] `patterns/array_matchable/let_array_destructure.ks` — **got:** `undefined name 'all.count.raw'`
- [x] `patterns/array_matchable/let_with_rest.ks` — **got:** `undefined name 'rest.count.raw'`
- [x] `patterns/array_matchable/prefix_rest_suffix.ks` — **got:** `undefined name 'middle.count.raw'`
- [x] `patterns/array_matchable/recursive_slice_destructuring.ks` — **got:** `undefined name 'rest'`
- [x] `patterns/array_matchable/rest_suffix_without_prefix.ks` — **got:** `undefined name 'rest.count.raw'`
- [x] `patterns/array_matchable/rest_with_binding.ks` — **got:** `undefined name 'rest.count.raw'`
- [x] `patterns/array_matchable/slice_array_pattern.ks` — **got:** `type mismatch: expected Slice[i64] got Array[i64]`

### Mutating-init body: `self.x = …` double-flagged as E201 + E005

Every init-body `self.field = value` fires both `cannot assign to immutable field 'x' [E201]` AND `initializer does not initialize all fields: 'x' [E005]`. Init-self-field assignment path is broken — both analyses see it as a no-op.

- [x] `validation/duplicate_callable/different_arity_with_same_label_start_is_valid.ks` — **got:** E201+E005 on lines 8,9
- [x] `validation/duplicate_callable/different_labels_is_valid_overload_init.ks` — **got:** E201+E005 on lines 8,9
- [x] `validation/duplicate_callable/same_labels_is_duplicate_init.ks` — **got:** E201+E005 on lines 8,9
- [x] `validation/duplicate_callable/two_protocols_same_init_label_different_types.ks` — **got:** E201+E005 on lines 16,17

### `str.unsafePtr` / `str.length` missing on String primitive

Primitive `str` lost these members somewhere; tests targeting pointer-interop fail.

- [x] `types/pointer/string_length_still_works.ks` — **got:** `no member 'length': str.length not found`
- [x] `types/pointer/string_unsafe_ptr_compiles.ks` — **got:** `no member 'unsafePtr': str.unsafePtr not found`
- [x] `types/pointer/string_unsafe_ptr_in_struct_field.ks` — **got:** `str.unsafePtr not found`, `str.length not found`
- [x] `types/pointer/string_unsafe_ptr_return_type.ks` — **got:** `str.unsafePtr not found`

### Runtime: global / computed property wrong value

The binary runs but produces wrong output. Related family to the codegen-link failures above.

- [x] `validation/properties_intended/global_let_initial_value.ks` — **expected stdout:** `7` · **got:** `-256` (uninitialized storage)
- [x] `validation/properties_intended/global_var_mutability_and_initial_value.ks` — **expected:** `0\n5` · **got:** `8663501056\n8660684288` (stack address leaked as value)
- [x] `validation/properties_intended/struct_computed_var_get_set.ks` — **expected:** `5\n9` · **got:** `5\n5` (setter not invoked)

### Dictionary default-hasher type arg leaks through unification/printer

`Dictionary[K, V]` unifies with `Dictionary[K, V, DefaultHasher]` should succeed (default arg), but the printer surfaces the third arg in diagnostics.

- [x] `types/type_operators/dictionary_operator/dictionary_get_value.ks` — **got:** `expected Dictionary[Int64, Int64] got Dictionary[Int64, Int64, DefaultHasher]`
- [x] `types/type_operators/dictionary_operator/dictionary_interchangeable_with_explicit.ks` — **got:** same
- [x] `types/type_operators/dictionary_operator/dictionary_type_basic.ks` — **got:** same

### Spurious dead-code / unreachable-code warnings

- [x] `expressions/returns/return_with_semicolon_followed_by_code.ks` — **got:** `unreachable code [E002]` on lines 8,9
- [x] `validation/dead_code/code_after_return_warns.ks` — **got:** `unreachable code [E002]` on line 9 (wrong line)
- [x] `validation/type_checking/while_with_wrong_return.ks` — **got:** `unreachable code [E002]` on line 10

### Protocol subscripts require a body (E608)

Subscript declarations inside protocol requirements shouldn't need a body; they should be abstract like methods.

- [x] `validation/duplicate_callable/different_labels_is_valid_overload_subscript.ks` — **got:** `subscript must have a body [E608]` on both overloads
- [x] `validation/duplicate_callable/same_labels_is_duplicate_subscript.ks` — **got:** `subscript must have a body [E608]` on both overloads

### `Prelude.*` path not resolvable

- [x] `builtins/matchable/generic_matchable.ks` — **got:** `cannot find type 'Prelude.Matchable' in this scope [E436]`, `no member 'matches': T.matches`
- [x] `expressions/throw/throw_with_try_pattern.ks` — **got:** `cannot find type 'Prelude' in this scope`, `undefined 'Prelude.ControlFlow.{Continue,Break}'`

### Shift operators leak `by:` label

Protocol signature for `<<`/`>>` expects unlabeled arg but source declares `by:`.

- [x] `expressions/protocol_operators/shift_left_operator_protocol.ks` — **got:** `wrong argument label: expected '_', got 'by'`
- [x] `expressions/protocol_operators/shift_right_operator_protocol.ks` — **got:** `wrong argument label: expected '_', got 'by'`

### `var (a, b) = tuple` destructuring — bindings reported as immutable

- [x] `patterns/let_destructuring/tuple_destructuring/var_tuple_destructure_mutable.ks` — **got:** `cannot assign to immutable variable 'a' [E200]`, `cannot assign to immutable variable 'b' [E200]`

### Binary-expression LHS of assignment produces wrong diagnostic

- [x] `validation/mutability/assign_to_binary_expression_fails.ks` — **got:** `unsupported binary operator '+'`

### Assign-to-field-on-immutable-receiver fires wrong diagnostic

- [x] `validation/mutability/assign_to_field_on_immutable_receiver.ks` — **got:** `cannot assign to immutable variable 's' [E200]` (should be "immutable field 'x'")

## False Negatives

### Syntax Sugar Errors (partial)

- [x] `expressions/protocol_operators/operator_without_protocol_conformance.ks` — **expected:** `add` · **got:** `does not conform to protocol: Number !: AddOperatorProtocol` (correct) + `no member 'add' on type 'Number'` (cascading; annotation matches the first, second is flagged unexpected)

### Match-expression diagnostics (partial)

- [x] `expressions/calls/method_calls/method_call_error_cases.ks` — **expected at line 15:** any error (fixed together with the parser-error entry in False Positives)

### Field-access / tuple-index diagnostics (partial)

- [x] `expressions/field_access/nonexistent_field_error.ks` — **expected:** `no member 'z' on type 'Point'`

### Field / variable mutability diagnostics on nested/field paths

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/assignment_validation/` — walks the LHS path; if any receiver on the chain is immutable, emits "cannot assign to immutable field". Complements `kestrel-semantic-tree-binder/src/diagnostics/assignment.rs` for the bind-time base-variable case.

- [x] `validation/mutability/nested_field_assignment_outer_immutable_fails.ks` — **expected:** `cannot assign to immutable field`
- [x] `validation/mutability/nested_field_assignment_receiver_immutable_fails.ks` — **expected:** `cannot assign to immutable field`

### Try-on-non-tryable-type diagnostic

> **lib1:** `kestrel-semantic-analyzers/src/analyzers/type_check/mod.rs` — `try x` desugar expects `Tryable` conformance, which surfaces the `tryExtract` diagnostic when the operand type doesn't have it.

- [x] `expressions/control_flow/try_on_non_tryable_type.ks` — **expected:** `tryExtract`

### Or-pattern inconsistent bindings

> **lib1:** `kestrel-semantic-tree-binder/src/body_resolver/patterns.rs` via `diagnostics/pattern.rs` — when lowering an or-pattern the binder joins the binding sets from each alternative and emits "inconsistent" if they differ in name or type.

- [x] `expressions/match/or_patterns/or_pattern_inconsistent_bindings_error.ks` — **expected:** `inconsistent`

### Move checker silent on non-Copyable double-move (with stdlib)

- [x] `memory_model/copy_semantics/not_copyable_move_semantics_with_stdlib.ks` — **expected at line 15:** `use of moved value` · **got:** none (pre-existing; was never passing)

## Stdlib

### Monomorphization witness gaps — protocol extension methods not in witness

Fixed 2026-04-20: witness generation never walked protocol extensions, so default implementations (e.g. `extend Iterator { func map(...) }`) were missing from witness tables for conforming types. Introduced a new `ProtocolMembers` / `ProtocolAssociatedTypes` / `ProtocolMembersByName` query set in `kestrel-name-res` that walks direct children, extension defaults, and inherited parent protocols (plus their extensions) in one pass. `witness_lower.rs` now calls `ProtocolMembers`; the old `collect_protocol_methods_recursive` helper and its filter-by-NodeKind logic are gone. Consumers never have to reassemble the walk from `ExtensionsFor` + `ConformingProtocols` again.

- [x] `stdlib/iterator/is_sorted_by_comparator.ks`
- [x] `stdlib/iterator/is_sorted_by_key.ks`
- [x] `stdlib/iterator/terminal_operations.ks`
- [x] `stdlib/iterator/try_for_each_adapter.ks`

Regressed: `stdlib/iterator/is_sorted_checks.ks` — collides on the overloaded `isSorted` name (two methods in `extend Iterator` both named `isSorted`). `IndexMap::insert` in the witness table keeps only one; the query refactor flipped which overload wins. Pre-existing overload-in-witness bug, not specific to this fix — tracked in the "Witness overload collision" bucket in # Stdlib.

### Derived-protocol bounds not propagated to generic params

Fixed 2026-04-20: two bugs in where-clause handling during type inference. (1) `where_clauses_in_context()` resolved `Equality` LHS using `self.owner` instead of the passed `context` entity, so type params like `T` in `flatten[U]() where T = Optional[U]` couldn't be found. (2) `solve_member` where-clause handling only searched the method's own type params; struct/extension type params and associated types (e.g. `Item` in `where Item = (A, B)`) were silently skipped. Added context parameter to `resolve_type_param_or_assoc`/`extract_associated_type_path`, and fallback to full `subs` map + `Associated` constraint emission for TypeAlias params.

- [x] `stdlib/optional/optional_flatten.ks`
- [x] `stdlib/iterator/unzip_iterator.ks`
- [x] `stdlib/iterator/filter_map_flatten.ks` — type inference fixed; now fails in the codegen bucket (listed in # Stdlib above)

### Codegen symbol not found (partial — fixed by witness-instantiation-collapse)

- [x] `stdlib/result/result_transforms.ks` — passes after witness-instantiation-collapse fix
- [x] `stdlib/string/replacement_and_splitting.ks` — passes after witness-instantiation-collapse fix
- [x] `stdlib/views/lines_view.ks` — passes after witness-instantiation-collapse fix

### Runtime exit-code failures (partial — fixed by witness-instantiation-collapse)

- [x] `stdlib/dictionary/dictionary_capacity_management.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/float32/float32_conversion.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/float64/float64_clamp_lerp_conversion_format.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/float64/float64_constructors_and_constants.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/int16/int16_bitwidth_and_conversion.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/int16/int16_boundaries_and_constants.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/int32/int32_bitwidth_and_conversion.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/int32/int32_boundaries_and_constants.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/int64/int64_byte_conversion_big_endian.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/int64/int64_byte_conversion_little_endian.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/uint8/uint8_bitwidth_and_conversion.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/uint16/uint16_bitwidth_and_conversion.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/uint32/uint32_bitwidth_and_conversion.ks` — fixed by witness-instantiation-collapse

### Explicit type args on method calls silently dropped (all fixed)

Historical: `generate.rs` discarded explicit type args from `HirExpr::MethodCall` (the `type_args` field was ignored via `..`). Methods like `cast[Int64]()` produced fresh unresolved TyVars instead of the specified types, causing `?` to leak into return types and downstream `!: NotEqual` errors. Both tests now pass after the witness-instantiation-collapse fix (2026-04-20).

- [x] `stdlib/memory/memory_allocator.ks` — fixed by witness-instantiation-collapse
- [x] `stdlib/memory/memory_raw_pointer.ks` — fixed by witness-instantiation-collapse
