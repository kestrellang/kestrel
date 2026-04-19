# Test Failures — 2026-04-19

Run: `file_tests --test-threads=1 --skip stdlib --skip function_as_value` on `feature/incremental-hecs`.
Result: **2370 passed · 214 failed · 204 filtered** (stdlib + known-hanging tests skipped).

---

# False Positives

Compiler rejects valid code, produces spurious diagnostics, emits wrong code, or runs code incorrectly.

## Inference: unresolved `?` infer-var leaks into type-mismatch diagnostic

The inference apply phase is printing raw `?` placeholders in type-mismatch errors instead of the resolved type. Root cause likely in solver's apply-substitutions / type-printer path.

- [ ] `inference/mod/inferred_type_mismatch_in_function_arg.ks` — **expected:** `does not conform to protocol` · **got:** `type mismatch: expected str got ?`
- [ ] `inference/mod/inferred_type_mismatch_in_return.ks` — **expected:** `does not conform to protocol` · **got:** `type mismatch: expected str got ?`
- [ ] `inference/mod/inferred_type_mismatch_with_usage.ks` — **expected:** `does not conform to protocol` · **got:** `type mismatch: expected str got ?`
- [ ] `types/generics/constraint_enforcement/explicit_type_arg_conflicts_with_inferred.ks` — **got:** `type mismatch: expected str got ?`
- [ ] `types/literals/array_mixed_types_error.ks` — **got:** `type mismatch: expected ? got ?`
- [ ] `validation/type_checking/struct_init_all_fields_wrong.ks` — **got:** `type mismatch: expected i64 got ?`
- [ ] `validation/type_checking/struct_init_bool_for_int.ks` — **got:** `type mismatch: expected i1 got ?`
- [ ] `expressions/match/type_inference/match_arms_must_have_same_type.ks` — **expected:** `type` · **got:** `type mismatch: expected ? got i64`
- [ ] `patterns/if_let/type_inference/if_let_branches_same_type.ks` — **expected:** `type` · **got:** `type mismatch: expected ? got i64`, `expected i64 got ?`
- [ ] `patterns/guard_let/divergence/guard_let_else_no_return_error.ks` — **expected:** `diverge` · **got:** `type mismatch: expected ? got ()` (alongside correct E003)

## Codegen: static/computed property entity not registered in symbol table

All fail during link with `unknown global entity` / `unknown function entity` for `Main.Foo._s`, `Main.Foo._v`, or `Main.globalComputedVar`. Entity(3523/3524) is the symbol id.

- [ ] `validation/properties_intended/enum_computed_var_get_set.ks` — **got:** `codegen/link failed: unknown global entity Entity(3524) (Main.Foo._v)`
- [ ] `validation/properties_intended/enum_static_computed_var_get_set.ks` — **got:** `unknown global entity Entity(3524) (Main.Foo._s)`
- [x] `validation/properties_intended/enum_static_let_initial_value.ks` — **got:** `unknown global Entity(3524)`
- [x] `validation/properties_intended/enum_static_var_mutability_and_initial_value.ks` — **got:** `unknown global Entity(3524)`
- [ ] `validation/properties_intended/global_computed_var_get_set.ks` — **got:** `call to unknown function entity Entity(3523) (Main.globalComputedVar)`
- [ ] `validation/properties_intended/struct_static_computed_var_get_set.ks` — **got:** `unknown global entity Entity(3523) (Main.Foo._s)`
- [x] `validation/properties_intended/struct_static_let_initial_value.ks` — **got:** `unknown global Entity(3523)`
- [x] `validation/properties_intended/struct_static_var_mutability_and_initial_value.ks` — **got:** `unknown global Entity(3523)`

## Runtime: global / computed property wrong value

The binary runs but produces wrong output. Related family to the codegen-link failures above.

- [x] `validation/properties_intended/global_let_initial_value.ks` — **expected stdout:** `7` · **got:** `-256` (uninitialized storage)
- [x] `validation/properties_intended/global_var_mutability_and_initial_value.ks` — **expected:** `0\n5` · **got:** `8663501056\n8660684288` (stack address leaked as value)
- [ ] `validation/properties_intended/struct_computed_var_get_set.ks` — **expected:** `5\n9` · **got:** `5\n5` (setter not invoked)

## Array rest-pattern bindings lower to `.count.raw` on undeclared symbol

Lowering of `[a, b, ...rest]` / `[all...]` emits `<binding>.count.raw` in MIR before the binding is actually introduced. All fail with `undefined name 'X.count.raw'`.

- [x] `patterns/array_matchable/capture_all_as_slice.ks` — **got:** `undefined name 'all.count.raw'`
- [x] `patterns/array_matchable/let_array_destructure.ks` — **got:** `undefined name 'all.count.raw'`
- [x] `patterns/array_matchable/let_with_rest.ks` — **got:** `undefined name 'rest.count.raw'`
- [x] `patterns/array_matchable/prefix_rest_suffix.ks` — **got:** `undefined name 'middle.count.raw'`
- [x] `patterns/array_matchable/recursive_slice_destructuring.ks` — **got:** `undefined name 'rest'`
- [x] `patterns/array_matchable/rest_suffix_without_prefix.ks` — **got:** `undefined name 'rest.count.raw'`
- [x] `patterns/array_matchable/rest_with_binding.ks` — **got:** `undefined name 'rest.count.raw'`
- [x] `patterns/array_matchable/slice_array_pattern.ks` — **got:** `type mismatch: expected Slice[i64] got Array[i64]`

## Mutating-init body: `self.x = …` double-flagged as E201 + E005

Every init-body `self.field = value` fires both `cannot assign to immutable field 'x' [E201]` AND `initializer does not initialize all fields: 'x' [E005]`. Init-self-field assignment path is broken — both analyses see it as a no-op.

- [x] `validation/duplicate_callable/different_arity_with_same_label_start_is_valid.ks` — **got:** E201+E005 on lines 8,9
- [x] `validation/duplicate_callable/different_labels_is_valid_overload_init.ks` — **got:** E201+E005 on lines 8,9
- [x] `validation/duplicate_callable/same_labels_is_duplicate_init.ks` — **got:** E201+E005 on lines 8,9
- [x] `validation/duplicate_callable/two_protocols_same_init_label_different_types.ks` — **got:** E201+E005 on lines 16,17

## Protocol subscripts require a body (E608)

Subscript declarations inside protocol requirements shouldn't need a body; they should be abstract like methods.

- [x] `validation/duplicate_callable/different_labels_is_valid_overload_subscript.ks` — **got:** `subscript must have a body [E608]` on both overloads
- [x] `validation/duplicate_callable/same_labels_is_duplicate_subscript.ks` — **got:** `subscript must have a body [E608]` on both overloads

## Static/`mutable var` property through type parameter read as immutable

- [ ] `codegen/generics/test_static_mutable_property_via_type_parameter.ks` — **expected:** no errors · **got:** `cannot assign to immutable field 'count' [E201]`

## ExpressibleByArrayLiteral doesn't fire for user types

- [ ] `builtins/literal_protocols/custom_type_with_array_literal.ks` — **expected:** no errors · **got:** `type mismatch: expected MyList got Array`

## `@builtin(.Copyable)` marker-protocol check too strict

E419 requires Copyable to be a marker protocol (no methods/types), but these tests declare it with methods/assoc types *expecting* the check — and E419 now fires unconditionally before the specific wording test expects.

- [ ] `builtins/protocols/copyable_on_protocol_with_associated_type.ks` — **got:** E419 `@builtin(.Copyable) must be a marker protocol`
- [ ] `builtins/protocols/copyable_on_protocol_with_method.ks` — **got:** E419 `@builtin(.Copyable) must be a marker protocol`

## Extension type-parameter not in where-clause scope

Extension's generic params (`extend S[T] where …`) don't make `T`, `U` visible inside `where` clause type references.

- [ ] `declarations/extensions/extension_type_param_not_in_scope.ks` — **got:** `cannot find type 'U' in this scope [E436]` at line 7
- [ ] `declarations/extensions/wrong_type_param_count.ks` — **got:** `cannot find type 'U' in this scope` at line 6

## Dictionary default-hasher type arg leaks through unification/printer

`Dictionary[K, V]` unifies with `Dictionary[K, V, DefaultHasher]` should succeed (default arg), but the printer surfaces the third arg in diagnostics.

- [x] `types/type_operators/dictionary_operator/dictionary_get_value.ks` — **got:** `expected Dictionary[Int64, Int64] got Dictionary[Int64, Int64, DefaultHasher]`
- [x] `types/type_operators/dictionary_operator/dictionary_interchangeable_with_explicit.ks` — **got:** same
- [x] `types/type_operators/dictionary_operator/dictionary_type_basic.ks` — **got:** same

## `str.unsafePtr` / `str.length` missing on String primitive

Primitive `str` lost these members somewhere; tests targeting pointer-interop fail.

- [x] `types/pointer/string_length_still_works.ks` — **got:** `no member 'length': str.length not found`
- [x] `types/pointer/string_unsafe_ptr_compiles.ks` — **got:** `no member 'unsafePtr': str.unsafePtr not found`
- [x] `types/pointer/string_unsafe_ptr_in_struct_field.ks` — **got:** `str.unsafePtr not found`, `str.length not found`
- [x] `types/pointer/string_unsafe_ptr_return_type.ks` — **got:** `str.unsafePtr not found`

## `Prelude.*` path not resolvable

- [x] `builtins/matchable/generic_matchable.ks` — **got:** `cannot find type 'Prelude.Matchable' in this scope [E436]`, `no member 'matches': T.matches`
- [x] `expressions/throw/throw_with_try_pattern.ks` — **got:** `cannot find type 'Prelude' in this scope`, `undefined 'Prelude.ControlFlow.{Continue,Break}'`

## Shift operators leak `by:` label

Protocol signature for `<<`/`>>` expects unlabeled arg but source declares `by:`.

- [x] `expressions/protocol_operators/shift_left_operator_protocol.ks` — **got:** `wrong argument label: expected '_', got 'by'`
- [x] `expressions/protocol_operators/shift_right_operator_protocol.ks` — **got:** `wrong argument label: expected '_', got 'by'`

## Range matchable — false non-exhaustive-match on total range patterns

- [ ] `patterns/range_matchable/range_from.ks` — **got:** `non-exhaustive match: missing _ [E305]`
- [ ] `patterns/range_matchable/char_range_from.ks` — **got:** `non-exhaustive match: missing _ [E305]`
- [ ] `patterns/range_matchable/char_range_exclusive.ks` — **got:** `Expected exit code 0, got 1` (compiler OK, runtime fail)
- [ ] `patterns/range_matchable/char_range_inclusive.ks` — **got:** `Expected exit code 0, got 1`
- [ ] `expressions/match/never_type/match_arm_with_break_in_loop.ks` — **got:** `non-exhaustive match: missing _ [E305]` (break→Never arm not counted as covering)

## Spurious unreachable-pattern / irrefutable-pattern warnings

Exhaustiveness pass flags these as unreachable/irrefutable when they aren't.

- [ ] `patterns/if_let/warnings/irrefutable_binding_pattern_warning.ks` — **got:** `unreachable pattern [E306]`
- [ ] `patterns/if_let/warnings/irrefutable_if_let_warning.ks` — **got:** `unreachable pattern [E306]`
- [ ] `patterns/exhaustiveness/overlapping_ranges.ks` — **got:** `unreachable pattern [E306]`
- [ ] `patterns/exhaustiveness/unreachable_after_wildcard.ks` — **got:** `irrefutable pattern in match arm makes 1 subsequent arm unreachable [E303]`
- [ ] `patterns/exhaustiveness/unreachable_array_rest.ks` — **got:** `Array is not defined`, `unsupported unary operator '-'`, non-exhaustive
- [ ] `patterns/pattern_types/nested_at_patterns_error.ks` — **got:** irrefutable E303 + unreachable E306
- [ ] `expressions/match/or_patterns/or_pattern_inconsistent_bindings_error.ks` — **got:** unreachable E306 (+ expected "inconsistent" missing)

## Spurious dead-code / unreachable-code warnings

- [x] `expressions/returns/return_with_semicolon_followed_by_code.ks` — **got:** `unreachable code [E002]` on lines 8,9
- [x] `validation/dead_code/code_after_return_warns.ks` — **got:** `unreachable code [E002]` on line 9 (wrong line)
- [x] `validation/type_checking/while_with_wrong_return.ks` — **got:** `unreachable code [E002]` on line 10

## Init delegation (`self.init(…)`) emits wrong diagnostics

- [ ] `declarations/delegating_initializers/delegation_outside_init.ks` — **got:** `wrong argument label: expected '_', got 'value'`, `cannot pass immutable self to mutating parameter [E203]` (+ expected `init` missing)
- [ ] `declarations/delegating_initializers/delegation_to_nonexistent_init.ks` — **got:** `wrong number of arguments: expected 0, got 1`
- [ ] `declarations/delegating_initializers/delegation_with_wrong_types.ks` — **got:** `no member 'init': Bad.init not found`, `duplicate initializer signature: init(_:) [E426]`

## `var (a, b) = tuple` destructuring — bindings reported as immutable

- [ ] `patterns/let_destructuring/tuple_destructuring/var_tuple_destructure_mutable.ks` — **got:** `cannot assign to immutable variable 'a' [E200]`, `cannot assign to immutable variable 'b' [E200]`

## Closure generic param inference E606 firing spuriously

- [ ] `expressions/closures/closure_with_generic_param_inferred.ks` — **got:** `could not infer type for closure parameter [E606]`

## Tuple arity error in parameter destructuring

- [ ] `declarations/parameter_destructuring/closure_tuple_arity_mismatch.ks` — **got:** `tuple pattern has 3 elements but type has 2 [E111]` (test expected a different diagnostic)

## Array literal with mixed wrong types — unification goes off the rails

- [ ] `validation/type_checking/array_mixed_multiple_wrong.ks` — **got:** `expected Bool got Int64`, `expected Float64 got Int64` (unifies element types pairwise instead of to single element type)

## Binary-expression LHS of assignment produces wrong diagnostic

- [x] `validation/mutability/assign_to_binary_expression_fails.ks` — **got:** `unsupported binary operator '+'`

## `type Alias = X` init-call and assoc-type projection regressions

- [ ] `declarations/type_aliases/type_alias_init_call.ks` — **got:** `no member '(subscript)': C.(subscript)`, `no member 'count': ?.count`
- [ ] `declarations/wacky_inference/nested_associated_type_projections.ks` — **got:** `no member 'baseValue': T.baseValue`
- [ ] `execution_graph/protocols/static_method_on_associated_type.ks` — **got:** `no member 'create': T.create`
- [ ] `validation/type_checking/tuple_index_with_associated_type_equality.ks` — **got:** `no member '0': Item.0`, `no member '1': Item.1`

## Operator without protocol conformance — wrong lookup

- [ ] `expressions/protocol_operators/operator_without_protocol_conformance.ks` — **got:** `no member 'add': Number.add not found`

## Try-operator member lookup

- [ ] `expressions/try_operator/try_on_non_tryable_type.ks` — **got:** `no member 'tryExtract': NotTryable.tryExtract`, `.fromResidual not found on i64`, non-exhaustive, unreachable

## Generic `not Copyable` type param — spurious Int32/T mismatches

- [ ] `memory_model/generic_copyability/type_parameter_with_not_copyable_can_be_moved_once.ks` — **got:** `type mismatch: expected Int32 got T`
- [ ] `memory_model/generic_copyability/type_parameter_with_not_copyable_use_after_move.ks` — **got:** `type mismatch: expected Int32 got T` (twice)

## For-loop over non-iterator `std.num.Int` not found

- [ ] `patterns/for_loops/for_loop_over_non_iterator_without_iter_method.ks` — **got:** `cannot find type 'std.num.Int' in this scope`, `cannot find type 'std' in this scope [E436]`

## Unexpected parser error in method_call_error_cases

- [ ] `expressions/calls/method_calls/method_call_error_cases.ks` — **got:** `expected '!', '=', or 11 others, found identifier` (line 16)

## Assign-to-field-on-immutable-receiver fires wrong diagnostic

- [x] `validation/mutability/assign_to_field_on_immutable_receiver.ks` — **got:** `cannot assign to immutable variable 's' [E200]` (should be "immutable field 'x'")

## Regressions on positive tests

- [ ] `builtins/intrinsics/panic_is_diverging.ks` — **expected:** no errors · **got:** `function 'unreachable' does not return a value on all code paths [E001]`
- [ ] `declarations/expression_bodied_functions/expression_bodied_function_with_where_clause.ks` — **expected:** no errors · **got:** `method 'double' has wrong return type for protocol 'Doubler' [E458]`

---

# False Negatives

Compiler fails to emit a diagnostic for code that should be rejected.

## Move / ownership / use-after-move checks not running

Borrow/move-checker not executing or not wired into bind→infer→validate pipeline.

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

- [ ] `validation/exhaustive_return/function_missing_return.ks` — **expected at line 8:** any error · **got at line 7:** E001 `does not return on all paths`
- [ ] `validation/exhaustive_return/if_else_chain_missing_final_else.ks` — **expected at line 12** · **got at line 7:** `expected i64 got ()`
- [ ] `validation/exhaustive_return/if_returns_else_falls_through.ks` — same pattern
- [ ] `validation/exhaustive_return/if_without_else_missing_return.ks` — same pattern
- [ ] `validation/exhaustive_return/loop_with_break_needs_return_after.ks` — **expected:** any error · **got:** none
- [ ] `validation/exhaustive_return/nested_if_inner_missing_else.ks` — **expected:** any error · **got:** `expected i64 got ()`
- [ ] `validation/exhaustive_return/statements_without_return.ks` — **expected at line 9** · **got at line 8:** E001
- [ ] `validation/exhaustive_return/while_loop_may_not_execute.ks` — **expected:** any error · **got:** none

## Visibility checks (public API surface uses private types)

- [ ] `validation/misc/protocol_method_with_private_param_in_public_protocol_errors.ks` — **expected:** `parameter type in 'handle' is less visible`
- [ ] `validation/misc/public_field_with_private_type_errors.ks` — **expected:** `has type less visible than the field`
- [ ] `validation/misc/public_function_with_private_parameter_type_errors.ks` — **expected:** `parameter type in 'process' is less visible`
- [ ] `validation/misc/public_function_with_private_return_type_errors.ks` — **expected:** `return type of 'getSecret' is less visible`
- [ ] `validation/misc/public_type_alias_with_private_underlying_errors.ks` — **expected:** `aliased type in 'Exposed' is less visible`
- [ ] `validation/visibility/private_method_not_visible_outside_struct.ks` — **expected:** `is private and not accessible from this scope`
- [ ] `expressions/field_access/private_field_access_error.ks` — **expected:** `is private`

## String-escape lexer diagnostics

Invalid `\x`, `\u{…}`, etc. aren't reported; lexer silently accepts bad escapes.

- [ ] `expressions/strings/ascii_escape_out_of_range.ks` — **expected:** `out of range`
- [ ] `expressions/strings/incomplete_hex_escape.ks` — **expected:** `invalid escape sequence`
- [ ] `expressions/strings/invalid_escape_sequence.ks` — **expected:** `invalid escape sequence`
- [ ] `expressions/strings/unicode_escape_empty_braces.ks` — **expected:** `invalid Unicode escape`
- [ ] `expressions/strings/unicode_escape_missing_brace.ks` — **expected:** `invalid Unicode escape`
- [ ] `expressions/strings/unicode_escape_out_of_range.ks` — **expected:** `invalid Unicode escape`
- [ ] `expressions/strings/unicode_escape_too_many_digits.ks` — **expected:** `invalid Unicode escape`

## Unknown-attribute warning

- [ ] `attributes/semantic/mixed_known_and_unknown_attributes.ks` — **expected:** `unknown attribute`
- [ ] `attributes/semantic/multiple_unknown_attributes_emit_multiple_warnings.ks` — **expected:** `unknown attribute` (lines 5,6)
- [ ] `attributes/semantic/unknown_attribute_emits_warning.ks` — **expected:** `unknown attribute`
- [ ] `attributes/semantic/unknown_attribute_with_args_emits_warning.ks` — **expected:** `unknown attribute`

## `let <refutable-pattern> = …` must be rejected

Refutable patterns in non-destructuring `let` produce only the downstream non-exhaustive-match error, not the required refutable-binding error.

- [ ] `patterns/let_destructuring/refutable_enum_pattern_error.ks` — **expected:** `refutable`
- [ ] `patterns/let_destructuring/refutable_literal_pattern_error.ks` — **expected:** `refutable`
- [ ] `patterns/let_destructuring/tuple_with_refutable_is_refutable.ks` — **expected:** `refutable`

## `if let` / `while let` binding scope leaks

Bindings introduced by `if let`/`while let` still resolve outside the scope.

- [ ] `patterns/if_let/scoping/binding_not_visible_after_if_let.ks` — **expected:** `undefined`
- [ ] `patterns/if_let/scoping/binding_not_visible_in_else.ks` — **expected:** `undefined`
- [ ] `patterns/while_let/scoping/binding_not_visible_after_loop.ks` — **expected:** `undefined`

## Init field-coverage across control flow

Initializer that sets `self.x` only in some branches should be rejected; check isn't flow-sensitive.

- [ ] `validation/initializers/init_only_in_while_body.ks` — **expected:** `does not initialize all fields`
- [ ] `validation/initializers/match_not_all_arms_initialize.ks` — **expected:** `does not initialize all fields`
- [ ] `validation/initializers/only_then_branch_initializes.ks` — **expected:** `does not initialize all fields`

## Dictionary literal requires `Hashable` key — protocol-conformance diagnostic

Tests expect "does not conform to protocol" (Hashable); compiler emits generic type-mismatch (or nothing) instead.

- [ ] `expressions/dictionary_literals/empty_dict_without_context.ks` — **expected:** `could not infer type`
- [ ] `expressions/dictionary_literals/inconsistent_key_types.ks` — **expected:** `does not conform to protocol`
- [ ] `expressions/dictionary_literals/inconsistent_value_types.ks` — **expected:** `does not conform to protocol`
- [ ] `expressions/dictionary_literals/key_type_mismatch.ks` — **expected:** `does not conform to protocol`
- [ ] `expressions/dictionary_literals/value_type_mismatch.ks` — **expected:** `does not conform to protocol`

## `for x in nonIterable` — missing Iterable conformance diagnostic

- [ ] `patterns/for_loops/for_loop_over_non_iterable.ks` — **expected:** `Iterable`
- [ ] `inference/mod/infer_array_element_type_mismatch.ks` — **expected:** `does not conform to protocol`

## Match-expression diagnostics

- [ ] `expressions/match/errors/duplicate_binding_name.ks` — **expected:** `duplicate`
- [ ] `expressions/match/errors/float_literal_in_pattern.ks` — **expected:** `float`
- [ ] `expressions/match/errors/unknown_enum_case.ks` — **expected:** `Blue` (unknown case name)
- [ ] `expressions/match/errors/wrong_enum_arity.ks` — **expected:** any error
- [ ] `expressions/match/errors/wrong_tuple_arity.ks` — **expected:** `arity`
- [ ] `expressions/match/guards/guard_must_be_bool.ks` — **expected:** `Bool`
- [ ] `expressions/calls/method_calls/method_call_error_cases.ks` — **expected at line 15:** any error

## Optional type diagnostics

- [ ] `types/optional/incompatible_type_no_promotion.ks` — **expected:** `type mismatch`
- [ ] `types/optional/nested_optional_no_promotion.ks` — **expected:** `type mismatch`
- [ ] `types/optional/non_optional_type_cannot_be_null.ks` — **expected:** `does not conform`

## Pointer / type-argument diagnostics

- [ ] `types/pointer/lang_ptr_empty_brackets_error.ks` — **expected:** `type argument`
- [ ] `types/pointer/lang_ptr_without_type_args_error.ks` — **expected:** `type argument`

## Struct arity / label diagnostics

- [ ] `declarations/structs/wrong_arity_too_few.ks` — **expected:** `has 2 field(s)`
- [ ] `declarations/structs/wrong_arity_too_many.ks` — **expected:** `has 2 field(s)`
- [ ] `declarations/structs/wrong_label_name.ks` — **expected:** `label`

## Struct vs protocol same name

- [ ] `validation/misc/struct_and_protocol_same_name_errors.ks` — **expected:** `duplicate` · **got:** E425 `'Foo' is already defined as a struct` (close semantics, wrong wording)

## Field-access / tuple-index diagnostics

Tests expect specific phrasing ("no member 'z' on type 'Point'", "cannot index into non-tuple type", "out of bounds"); compiler emits the generic member-lookup error instead.

- [ ] `expressions/field_access/member_access_on_primitive_type_error.ks` — **expected:** `cannot access member on type`
- [ ] `expressions/field_access/nonexistent_field_error.ks` — **expected:** `no member 'z' on type 'Point'`
- [ ] `validation/type_checking/tuple_index_on_non_tuple.ks` — **expected:** `cannot index into non-tuple type`
- [ ] `validation/type_checking/tuple_index_out_of_bounds.ks` — **expected:** `out of bounds`

## Field / variable mutability diagnostics on nested/field paths

- [x] `validation/mutability/nested_field_assignment_outer_immutable_fails.ks` — **expected:** `cannot assign to immutable field`
- [x] `validation/mutability/nested_field_assignment_receiver_immutable_fails.ks` — **expected:** `cannot assign to immutable field`

## Self in wrong context

- [ ] `expressions/calls/method_calls/self_in_free_function_error.ks` — **expected:** `cannot use 'self' in free function`
- [ ] `expressions/calls/method_calls/self_in_static_method_error.ks` — **expected:** `cannot use 'self' in static method`

## Primitive-method name-hint

- [ ] `expressions/calls/method_calls/primitive_methods_errors.ks` — **expected:** `primitive method 'toString' on 'I64' must be called`

## Setter required by protocol but only getter provided

- [ ] `declarations/computed_properties/protocol_requires_setter_but_only_getter_provided.ks` — **expected:** `setter`

## Try-on-non-tryable-type diagnostic

- [ ] `expressions/control_flow/try_on_non_tryable_type.ks` — **expected:** `tryExtract`

## Or-pattern inconsistent bindings

- [ ] `expressions/match/or_patterns/or_pattern_inconsistent_bindings_error.ks` — **expected:** `inconsistent`

## Empty array literal requires type annotation

- [ ] `expressions/paths/empty_array_requires_type_annotation.ks` — **expected:** `could not infer type`
