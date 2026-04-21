# Fixed

Tests previously listed in `test-errors.md` as failing, now passing. Grouped by their original category so the surrounding context (analyzer location, root-cause notes, regression tracking) stays intact.

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

## Stdlib

### Monomorphization witness gaps — protocol extension methods not in witness

Fixed 2026-04-20: witness generation never walked protocol extensions, so default implementations (e.g. `extend Iterator { func map(...) }`) were missing from witness tables for conforming types. Introduced a new `ProtocolMembers` / `ProtocolAssociatedTypes` / `ProtocolMembersByName` query set in `kestrel-name-res` that walks direct children, extension defaults, and inherited parent protocols (plus their extensions) in one pass. `witness_lower.rs` now calls `ProtocolMembers`; the old `collect_protocol_methods_recursive` helper and its filter-by-NodeKind logic are gone. Consumers never have to reassemble the walk from `ExtensionsFor` + `ConformingProtocols` again.

- [x] `stdlib/iterator/is_sorted_by_comparator.ks`
- [x] `stdlib/iterator/is_sorted_by_key.ks`
- [x] `stdlib/iterator/terminal_operations.ks`
- [x] `stdlib/iterator/try_for_each_adapter.ks`

Regressed: `stdlib/iterator/is_sorted_checks.ks` — collides on the overloaded `isSorted` name (two methods in `extend Iterator` both named `isSorted`). `IndexMap::insert` in the witness table keeps only one; the query refactor flipped which overload wins. Pre-existing overload-in-witness bug, not specific to this fix — tracked in the "Witness overload collision" bucket in `test-errors.md` # Stdlib.

### Derived-protocol bounds not propagated to generic params

Fixed 2026-04-20: two bugs in where-clause handling during type inference. (1) `where_clauses_in_context()` resolved `Equality` LHS using `self.owner` instead of the passed `context` entity, so type params like `T` in `flatten[U]() where T = Optional[U]` couldn't be found. (2) `solve_member` where-clause handling only searched the method's own type params; struct/extension type params and associated types (e.g. `Item` in `where Item = (A, B)`) were silently skipped. Added context parameter to `resolve_type_param_or_assoc`/`extract_associated_type_path`, and fallback to full `subs` map + `Associated` constraint emission for TypeAlias params.

- [x] `stdlib/optional/optional_flatten.ks`
- [x] `stdlib/iterator/unzip_iterator.ks`
- [x] `stdlib/iterator/filter_map_flatten.ks` — type inference fixed; now fails in the codegen bucket (listed in `test-errors.md` # Stdlib above)

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

### `Self.Item` in protocol extension leaked to codegen as `AssociatedProjection(Named(protocol), …)` (2026-04-21)

Bare `Item` inside `extend Iterator { func collect() -> Array[Item] }` lowered to `AssocProjection { base: HirTy::Protocol(Iterator), assoc: Item }`. HIR has no `SelfType` variant, so name-resolution collapses `Self` inside `extend Protocol` to the protocol entity itself. MIR then produced `AssociatedProjection { base: Named(Iterator), protocol: Iterator, name: "Item" }` — codegen's `substitute_type_with_self` couldn't map the base back to the concrete self, the projection layout defaulted to `ptr` (8 bytes), and sub-i64 `Item` types (UInt8, Char, Grapheme, …) silently read back as garbage. Fix at MIR lowering: when HIR's `AssocProjection.base` is the owning protocol, emit `Named(assoc_typealias, [])` — the existing `resolve_assoc_type_substs` pass resolves that via witness lookup. Also plumbed `self_type` into `resolve_assoc_type_substs` (subst candidates tried first so `Array.init[I](from: I)` resolves `I.Iter` via `I`, not `Self`). Principled fix deferred — see `kestrel-hir-lower/src/AGENTS.md`.

- [x] `stdlib/views/bytes_view_iter.ks` — was exit 2, wrong collected bytes
- [x] `stdlib/views/chars_view_iter_and_count.ks` — was exit 3
- [x] `stdlib/array/misc_extensions.ks` — was MIR lowering panic
- [x] `stdlib/iterator/min_by_max_by.ks` — was exit 2
- [x] `stdlib/iterator/reduce_adapter.ks` — was exit 2
- [x] `stdlib/iterator/try_fold_adapter.ks` — was undeclared-symbol link error

### Integer literal overflow silently returned 0 (2026-04-21)

`parse_int` in `lib2/kestrel-hir-lower/src/pat.rs` used `i64::from_str` and `unwrap_or(0)`, so any integer literal above `i64::MAX` (e.g. `UInt64.maxValue = 18446744073709551615`, `2^63 = 9223372036854775808`) silently parsed to `0`. All three UInt64 runtime failures had the same shape: a literal past the i64 range was read as zero, so `UInt64.maxValue.isZero` was true, `maxVal.addChecked(one)` returned `Some(1)`, and `highBit.leadingZeros` was 64 instead of 0. Fix: fall back to `u64::from_str_radix` on overflow and reinterpret the bit pattern as i64 — applies to decimal, hex, octal, and binary literals.

- [x] `stdlib/uint64/uint64_bitwidth_and_conversion.ks` — was exit 5 (`highBit.leadingZeros != 0`)
- [x] `stdlib/uint64/uint64_boundaries_and_constants.ks` — was exit 7 (`maxVal.isZero`)
- [x] `stdlib/uint64/uint64_overflow_behavior.ks` — was exit 3 (`maxVal.addChecked(one).isSome()`)

### Witness overload collision — `isSorted` arity-0 dropped from witness table (2026-04-21)

Protocol extension with two same-named methods (`isSorted()` and `isSorted(by:)` on `Iterator`) collided in the witness table because `IndexMap::insert` keyed only on method name. Calls to the dropped overload failed with Cranelift arg-count errors. Resolved as collateral of the 2026-04-21 fixes in this session.

- [x] `stdlib/iterator/is_sorted_checks.ks` — was `mismatched argument count: got 2, expected 3`

### Unresolved method-level type parameters now reported instead of leaking `MirTy::Error` (2026-04-21)

`tryFold[Acc, E]` called with a closure that only returns `.Ok(...)` left `E` unbound. Lib1 would have silently defaulted `E` to `Never` via `apply_never_defaults` (solver.rs:101). Lib2 didn't port that pass, so the unresolved `E` leaked through inference as `MirTy::Error`, the mangler encoded `X` in the instantiation symbol, and monomorphize phantom-skipped it — producing a link-time "call to undeclared function".

Fix: skip the never-default entirely. Added a new `InferError::UnresolvedTypeParam` variant and a phase-4 pass in `lib2/kestrel-type-infer/src/solver.rs` (`report_unresolved_type_params`) that walks `ctx.type_args`, resolves each TyVar, and emits a diagnostic at the call site for any still-`Unresolved { literal: None }` slot. Poisons the TyVar so downstream constraints absorb silently. `try_fold_adapter.ks` now annotates the binding explicitly; `try_fold_unconstrained_error_type.ks` is the new diagnostic test that asserts the error fires when the annotation is missing.

- [x] `stdlib/iterator/try_fold_adapter.ks` — was `call to undeclared function: tryFold`; now passes after binding-type annotation

### Dictionary `subscript(key:inserting:)` removed — stdlib API contract mismatch (2026-04-21)

The `inserting:` subscript's doc-comment promised "If the key doesn't exist, the default is inserted and returned," but the getter never inserted — only the setter did. Commit 59de94b8 (2026-04-03) had removed the in-getter `self.insert(…)` to silence an analyzer complaint that a non-`mutating get` was mutating `self`. That broke the documented contract, so `dictionary_subscripts.ks` (migrated from lib1, where the getter still inserted) exited 6 at `if dict.contains(50) == false`.

Design choice: drop `inserting:` rather than add `mutating get`. Without mutating get, `inserting:` and the existing `default:` subscript are behaviorally identical for bare reads, and `default:` already supports the compound-assign accumulator pattern (`counts(k, default: 0) += 1`) via its setter. A subscript read that silently inserts is the least defensible version of the API — surprising, hurts debuggability, and an explicit mutating method (`getOrInsert`) is the right shape if real use cases emerge.

Fix: deleted the `subscript(key:inserting:)` block from `lang/std/collections/dictionary.ks` and simplified the test to exercise `default:` + `unwrap:` only.

- [x] `stdlib/dictionary/dictionary_subscripts.ks` — was exit 6

### `@fileconstant` dropped during lib2 MIR lowering (2026-04-21)

`@fileconstant("data/…bin")` on stdlib unicode case-mapping statics was parsed into an `Attributes` ECS component but lib2's MIR lowering never read it — codegen took the zero-init path, so `UPPER_STAGE1`-class `LiteralSlice` globals ended up in `__DATA.__bss` with null data pointers. Any subscript read segfaulted; the ASCII fast path in `toUppercase` hid it for `'a'→'A'` but `'A'.toUppercase()` tripped straight into the subscript. Fix at `lib2/kestrel-mir-lower/src/static_lower.rs::extract_file_constant`: read the `Attributes` component, walk `FileId → FilePath` for the source file's directory, extract element type from `LiteralSlice[T]`'s `Named.type_args[0]`, populate `StaticDef.file_constant_data`. Also moved `FilePath` from `kestrel-compiler2::components` to `kestrel-ast-builder::components` so MIR-lower can read it without a cyclic dep. Codegen's existing rodata-embed path was already correct.

- [x] `stdlib/char/char_case_conversion.ks` — was exit -1 SIGSEGV on `'A'.toUppercase()` (also previously on the macOS-UNE skip list)
- [x] `stdlib/string/case_conversion.ks` — was exit 7 (`titlecased` used the same broken case-mapping tables)
- [x] `stdlib/views/graphemes_view.ks` — was exit 1 (grapheme break tables `GBP_STAGE1`/`GBP_STAGE2` are also `@fileconstant`)
