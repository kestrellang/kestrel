# Fuzz report — commit 3fea2041

Reproducers: `temp/fuzz/<area>/min/<file>`. Build: `./target/release/kestrel build <file> -o /tmp/x && /tmp/x`.

**Severity**
- **Critical**: valid-looking code silently miscompiles or is memory-unsafe (wrong values, double drop, use-after-free, segfault).
- **High**: compiler crash or ICE on realistic code, or invalid code accepted that leads to undefined behaviour.
- **Medium**: valid code rejected, invalid code accepted but harmless, or a realistic compile-time blowup.
- **Low**: diagnostic quality, pathological inputs, or spec ambiguity.

**Category**
- Miscompile
- ICE (internal error, panic, verifier or backend failure)
- Stack overflow / hang
- Accepts invalid
- Rejects valid
- Diagnostic
- Stdlib

## Critical (25)
| # | Bug | Category | Repro |
|---|---|---|---|
| C1 | Integer literal and range patterns truncated to 32 bits (`5000000000` matches `705032704`, `0..=i64max` misses values, exhaustive i64 match traps) | Miscompile | codegen/int64_match_truncated_i32, codegen/u64_range_pattern, enums_match/int_literal_pattern_truncated, range_bound_over_u32, u64_high_range, i64max_range_trap |
| C2 | Assignment through `arr(i).field.x` or `arr(i).0` silently dropped | Miscompile | codegen/array_elem_nested_field_assign_lost |
| C3 | Tuple rest pattern `(a, .., b)` binds the wrong element | Miscompile | enums_match/tuple_rest_pattern_wrong_binding |
| C4 | Labels in enum payload patterns are ignored; fields bind by position | Miscompile | enums_match/labeled_payload_reorder |
| C5 | Two conformances to one generic protocol (`Conv[Int64]`, `Conv[String]`): generic dispatch segfaults | Miscompile | generics_protocols/multi_generic_proto_conformance_segv |
| C6 | Same setup, direct call overloaded only by return type: segfault | Miscompile | generics_protocols/return_type_overload_direct_segv |
| C7 | Closure parameter returning a String concat corrupts the heap (`malloc(): unaligned tcache chunk`) | Miscompile | closures/miscompile_closure_param_returns_string_concat, miscompile_escaping_capture_call_string_concat |
| C8 | `consuming` closure loses writes to a captured var's fields; an Array append hits the outer buffer | Miscompile | closures/miscompile_consuming_closure_field_assign_lost, miscompile_consuming_closure_array_append_hits_outer |
| C9 | `escaping` closure that captures a not-Copyable local reads it after it is dropped | Miscompile | ownership/escaping_closure_noncopyable_capture_dropped_early |
| C10 | Assigning a field inside `deinit` drops the old value twice and leaks the new one | Miscompile | ownership/deinit_field_reassign_double_drop |
| C11 | Conformance accepted with the wrong parameter type; the call passes Int64 as String and segfaults | Accepts invalid | generics_protocols/param_type_mismatch_conformance |
| C12 | `mutating` impl satisfies a non-mutating requirement, so a `let` binding gets mutated | Accepts invalid | generics_protocols/mutating_impl_for_nonmutating_req |
| C13 | Moving out of a `mutating` parameter accepted, so a double drop | Accepts invalid | ownership/move_out_of_mutating_param |
| C14 | `deinit r` on a borrowed parameter accepted, so a double drop | Accepts invalid | ownership/deinit_borrowed_param_double_drop |
| C15 | `consuming` closure built in a loop moves its capture on every iteration, so a triple drop | Accepts invalid | ownership/consuming_closure_in_loop_moves_capture_repeatedly |
| C16 | Array literal of not-Copyable elements accepted; `for in` traps with SIGILL | Accepts invalid | ownership/array_literal_noncopyable_forin_sigill |
| C17 | A ref-carrying struct stored into an outer `var` escapes its scope (dangling) | Accepts invalid | ownership/ref_escapes_inner_scope_via_outer_var |
| C18 | View closure escapes inside an `Optional`, so a segfault | Accepts invalid | closures/accepts_invalid_view_closure_in_optional_returned |
| C19 | View closure escapes through an `if` expression join, so a segfault | Accepts invalid | closures/accepts_invalid_view_closure_if_join_returned |
| C20 | View closure escapes through reassignment inside an `if` or loop, so a segfault | Accepts invalid | closures/accepts_invalid_view_closure_conditional_reassign_returned, _loop_reassign_returned |
| C21 | View closure escapes through an un-annotated array literal, so a segfault | Accepts invalid | closures/accepts_invalid_view_closure_via_array_literal_returned |
| C22 | View closure escapes through a call result, so a segfault | Accepts invalid | closures/accepts_invalid_view_closure_through_call_result |
| C23 | Moving a field out of a type with `deinit` silently skips the owner's deinit | Miscompile | ownership/field_move_skips_owner_deinit |
| C24 | Discarded temporary dropped at scope end instead of statement end (violates drop-semantics.md rule 6) | Miscompile | ownership/temp_not_dropped_at_stmt_end |
| C25 | Cranelift picks the wrong `conv` overload, giving a verifier error (LLVM builds it; probably the same root cause as C5) | ICE | generics_protocols/multi_conv_bool_verifier |

## High (26)
| # | Bug | Category | Repro |
|---|---|---|---|
| H1 | Bootstrap: `Iterator.iter()` instantiated at `EnumerateIterator<non-Copyable>` fails post-mono verify (**blocks CI on PR #233**) | ICE | lang/flock bootstrap |
| H2 | Nested struct or enum inside a struct (generic or not): `has no MonoStruct/MonoEnum`; nested enum case panics | ICE | frontend/nested_struct_basic, nested_enum_case, generics_protocols/nested_struct_in_generic_ice |
| H3 | Generic struct with an `escaping` closure field over an associated type: "Copyable contains non-Copyable field" (blocks Map/Filter adapters) | ICE | generics_protocols/generic_struct_escaping_field_copyable_ice |
| H4 | Moving a value while a borrow is live reaches the OSSA verifier instead of E498 (also `f(x, x)` with mutating plus consuming, and a dead ref before `match`) | ICE | ownership/ossa_move_while_ref_live, ossa_consume_and_mutate_same_arg, ossa_match_consumes_while_dead_ref |
| H5 | `while let` over a not-Copyable var reassigned or broken out of in the body: "address already uninit" | ICE | ownership/while_let_reassign_scrutinee_ossa_uninit, enums_match/while_let_noncopy_reassign_ice |
| H6 | String literal patterns on an owned scrutinee (match, tuple, Optional, array): "consumed more than once" or "never consumed" | ICE | enums_match/temp_string_two_literal_arms_ice, tuple_string_literal_owned_ice, array_pattern_string_literal_ice |
| H7 | Matching a not-Copyable value twice gives an OSSA ICE instead of E500 | ICE | enums_match/match_after_consume_ice |
| H8 | `.Some(var r)` binding of a not-Copyable payload | ICE | enums_match/var_binding_noncopy_payload_ice |
| H9 | Not-Copyable match where every arm returns, inside `for`: Cranelift jump argument mismatch / LLVM verify failure | ICE | enums_match/noncopy_match_all_return_in_for_verifier |
| H10 | Same loop shape with the match under `if`: "@owned never consumed" | ICE | enums_match/noncopy_match_in_loop_if_ossa_ice |
| H11 | Struct patterns in match arms: "TupleExtract on non-tuple" | ICE | enums_match/struct_pattern_in_match_codegen |
| H12 | `two(x, { x })` with a mutating first argument: "active mut borrow" OSSA ICE | ICE | closures/ice_ossa_mut_borrow_and_view_closure_same_call |
| H13 | `gen[T: Cloneable]` instantiated at an escaping closure: post-mono "does not implement clone" | ICE | closures/ice_escaping_closure_clone_generic |
| H14 | Stored `var` in an `extend` block accepted, then Cranelift panics with index out of bounds | ICE | frontend/extend_stored_property |
| H15 | `struct S { var x: S? }` fails post-mono verify (should be E-diagnostic or work) | ICE | frontend/recursive_optional |
| H16 | `deinit r;` as the only statement panics in move_tracking.rs:2278 | ICE | ownership/deinit_param_as_only_stmt_panic |
| H17 | Implicit member call to an instance method `g(.round())`: OSSA argument-count ICE | ICE | frontend/implicit_member_instance_method |
| H18 | `@main` with a parameter: OSSA argument-count ICE instead of a diagnostic | ICE | codegen/ice_main_with_param |
| H19 | `S.v` (instance field through the type name) reaches codegen: "global not found in statics" | ICE | frontend/instance_field_on_type |
| H20 | `Int8(from: Int8)` passes type-check and then fails post-mono verify | ICE | codegen/postmono_same_type_from |
| H21 | `return return` gives OSSA "operand never defined" | ICE | frontend/return_return |
| H22 | Polymorphic recursion `recur([x], n-1)` hangs the compiler | Stack overflow / hang | generics_protocols/polymorphic_recursion_hang |
| H23 | Type alias cycle (`type A = B; type B = A`, `type A = S[A]`) overflows the compiler's stack; `type A = Array[A]` silently accepted | Stack overflow / hang | frontend/alias_cycle_use, alias_generic_cycle2, alias_generic_cycle |
| H24 | Default argument calling its own function overflows the compiler's stack | Stack overflow / hang | frontend/default_arg_recursion |
| H25 | Bool-tuple match compile time doubles per element (15 elements = 128s) | Stack overflow / hang | enums_match/bool_tuple_match_exponential_compile |
| H26 | `Float64.round()` rounds ties to even; the docs say away from zero | Miscompile | codegen/float_round_ties_even |

## Medium (27)
| # | Bug | Category | Repro |
|---|---|---|---|
| M1 | Exhaustive Bool and enum tuple matches reported as missing `_` | Rejects valid | enums_match/bool_tuple_exhaustive_rejected |
| M2 | Full unsigned ranges (`0..=255` on UInt8) not exhaustive | Rejects valid | enums_match/uint8_full_range |
| M3 | Empty match on an empty enum: E304 "inhabited" | Rejects valid | enums_match/empty_enum_match_rejected |
| M4 | Struct pattern inside an enum payload `.Some(P { x, y })` is a parse error | Rejects valid | enums_match/struct_pattern_inside_enum_payload_parse_error |
| M5 | `deinit x;` only parses at the top level of a function body | Rejects valid | ownership/deinit_stmt_in_nested_block_parse_error |
| M6 | Field move from an owned local (`let x = p.a`) and tuple `p.0` from a consuming param rejected as "out of a borrow" | Rejects valid | ownership/field_move_from_owned_local_rejected, tuple_field_move_from_consuming_param_rejected |
| M7 | `match r` on a `&E` binding rejected even though the arms only read | Rejects valid | ownership/match_on_ref_binding_noncopyable_rejected |
| M8 | E507 freeze is not field-granular (consuming `p.b` is blocked by a capture of `p.a.v`) | Rejects valid | closures/rejects_valid_e507_disjoint_field_consume |
| M9 | E507 still active after an immediately-invoked closure | Rejects valid | closures/rejects_valid_e507_after_iife |
| M10 | Sub-protocol extension can't see parent protocol methods | Rejects valid | generics_protocols/subproto_ext_parent_method |
| M11 | Protocol extension can't pass `self` to a `where T: P` function | Rejects valid | generics_protocols/proto_ext_self_to_generic_fn |
| M12 | `Box.make(3)` on an unspecialized generic leaks the rigid `T` | Rejects valid | generics_protocols/static_on_unspecialized_generic_leaks_T |
| M13 | Nested generic loses the outer type parameter | Rejects valid | generics_protocols/nested_generic_outer_param_lost |
| M14 | `T.A = U.A` same-type constraint doesn't carry conformance | Rejects valid | generics_protocols/assoc_same_type_equal_not_propagated |
| M15 | Generic tuple alias `type Pair[T] = (T, T)` not expanded (documented in write-kestrel) | Rejects valid | generics_protocols/generic_tuple_alias_mismatch |
| M16 | `extend [T]: P` sugar fails where `extend Array[T]` works | Rejects valid | generics_protocols/array_sugar_conditional_extension |
| M17 | `"\(x[0])"` subscript in interpolation is a parse error | Rejects valid | frontend/interp_subscript |
| M18 | `t.1.0` nested tuple index lexed as a float | Rejects valid | codegen/parse_nested_tuple_index |
| M19 | Payload pattern with a wrong or duplicate label accepted (`.A(z: n)` for `A(x:)`) | Accepts invalid | enums_match/payload_label_wrong_name_accepted |
| M20 | `let g: Int64 = g` and mutually recursive globals accepted | Accepts invalid | frontend/global_self_init |
| M21 | `extend Int64: NopeProto {}` (undeclared protocol) accepted | Accepts invalid | frontend/extend_unknown_protocol |
| M22 | `import Nope.Thing` accepted | Accepts invalid | frontend/import_unknown_module |
| M23 | `E.a.b` (case access on a case) type-checks as `()` | Accepts invalid | frontend/enum_case_on_case |
| M24 | `struct S: S {}` accepted | Accepts invalid | frontend/self_conform |
| M25 | 10,000 `let`s take 99s to compile (quadratic) | Stack overflow / hang | frontend/many_lets_quadratic |
| M26 | `Float64.abs(-0.0)` returns `-0` | Stdlib | codegen/float_abs_negzero |
| M27 | Unterminated block comment: no error, only E618 pointed at a stdlib file | Diagnostic | frontend/unterminated_comment |

## Low (14)
| # | Bug | Category | Repro |
|---|---|---|---|
| L1 | Compiler stack overflow on 3,000-deep blocks, if-chains, closures or whiles (move_tracking) | Stack overflow / hang | frontend/block_deep etc. |
| L2 | Compiler stack overflow on 3,000-deep binops, calls, method chains or matches (MIR lowering) | Stack overflow / hang | frontend/binop_deep etc. |
| L3 | Compiler stack overflow on a deeply nested tuple type | Stack overflow / hang | frontend/deep_tuple_nest |
| L4 | `Array[...]` nested 3,000 deep runs out of memory | Stack overflow / hang | frontend/type_nest_3000_oom |
| L5 | Inference depth limit around 512 gives a bogus E109 | Rejects valid | frontend/unary_depth_512 |
| L6 | Duplicate external labels `f(a x:, a y:)` accepted | Accepts invalid | frontend/dup_labels |
| L7 | Unknown attribute `@nope` silently accepted | Accepts invalid | frontend/unknown_attr |
| L8 | Format specifiers ignored for Bool width and float hex | Stdlib | frontend/format_width_bool_ignored, format_hex_float_ignored |
| L9 | Float `:.N` precision switches between decimals and significant digits | Stdlib | codegen/fmt_float_precision_sci |
| L10 | `Int64.minValue.gcd(6)` returns -2 | Stdlib | codegen/gcd_minvalue_negative |
| L11 | Backtick identifier error leaks the internal name `$let_tmp` | Diagnostic | frontend/backtick_ident |
| L12 | `(return 1) + 2` gives "no member 'add' on Never" | Diagnostic | frontend/never_operand_add |
| L13 | UTF-8 BOM at file start rejected | Rejects valid | frontend/bom |
| L14 | Negative literal patterns `-1 =>` unsupported | Rejects valid | enums_match/neg_literal_pattern |

**Totals:** 25 Critical, 26 High, 27 Medium, 14 Low = 92.

By category (one primary category per bug):

| Category | Count |
|---|---|
| ICE | 22 |
| Rejects valid | 21 |
| Accepts invalid | 20 |
| Miscompile | 13 |
| Stack overflow / hang | 9 |
| Stdlib | 4 |
| Diagnostic | 3 |
| **Total** | **92** |

## Added after review (3)
| # | Bug | Category | Severity | Repro |
|---|---|---|---|---|
| A1 | Returning an `escaping` closure parameter is rejected: `func adapt(f: escaping (Int64)->Int64) -> (Int64)->Int64 { f }` gets E494 "captures local `f`". Returning a normal closure parameter is accepted. | Rejects valid | Medium | closures/min/rejects_valid_return_escaping_param |
| A2 | A generic call to a protocol-extension method that is not a requirement dispatches to the concrete type's same-named method (returns 50, not the extension's 1). Static dispatch would expect the extension. | Miscompile (semantics) | High | generics_protocols/min/ext_method_dispatches_to_concrete_shadow |
| A3 | E494 on an `if`/`else` returning refs to two locals prints one error and then "OSSA verification failed with 2 error(s)". The second error is never shown, and the message drops the local's name ("borrows local, which…"). | Diagnostic | Low | ownership/min/e494_reports_phantom_ossa_errors |

New totals: 25 Critical, 27 High, 28 Medium, 15 Low = **95**.
