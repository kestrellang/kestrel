# Test Suite Failures Analysis

45 tests failing in kestrel-test-suite. Analysis below.

## Issue 1: Type Mismatch Error Messages (5 tests)

Tests expecting "type mismatch" error are getting "does not conform to protocol `ExpressibleByIntegerLiteral`" instead.

**Affected tests:**
- `inference::inference_errors::inferred_type_mismatch_in_return`
- `inference::inference_errors::inferred_type_mismatch_in_function_arg`
- `inference::inference_errors::inferred_type_mismatch_with_usage`
- `builtins::literal_protocols::errors::literal_without_conformance`
- `builtins::literal_protocols::errors::wrong_literal_type`

**Root cause:** The error message format changed - instead of reporting "type mismatch", the compiler now reports the more specific protocol conformance error.

---

## Issue 2: Call Resolution Regression (Many tests)

Multiple tests are failing with messages like:
- "no matching overload found for call to `C`"
- "no matching overload found for call to `add`"
- "no matching overload found for call to `Counter`"

**Affected tests:**
- `declarations::type_aliases::regression::type_alias_init_call`
- `declarations::type_aliases::regression::type_alias_static_method_access`
- `declarations::type_aliases::regression::type_alias_instance_method_call`
- `expressions::calls::function_calls::function_calls::call_function_with_multiple_params`
- `expressions::calls::function_calls::function_calls::nested_function_calls_two_levels`
- `expressions::operators::arithmetic_operators::integer_arithmetic_operations`
- `expressions::operators::associativity::left_associative_arithmetic`
- `expressions::operators::bitwise_operators::all_bitwise_operators`
- `expressions::operators::combined_with_variables::operators_with_function_parameters`
- `expressions::protocol_operators::arithmetic_protocols::remainder_operator_protocol`
- `expressions::protocol_operators::comparison_protocols::greater_than_or_equals_operator_protocol`
- `expressions::protocol_operators::comparison_protocols::less_than_or_equals_operator_protocol`
- `declarations::enums::unlabeled_cases::either_enum_unlabeled`
- `declarations::enums::unlabeled_cases::multiple_unlabeled_pattern_matching`
- `declarations::enums::unlabeled_cases::nested_unlabeled_enums`
- `declarations::enums::unlabeled_cases::recursive_enum_unlabeled`
- `declarations::enums::unlabeled_cases::result_enum_unlabeled`
- `declarations::enums::unlabeled_cases::three_unlabeled_parameters`
- `declarations::enums::unlabeled_cases::unlabeled_case_pattern_matching`
- `declarations::enums::error_type_mismatch::wrong_type_for_associated_value`
- `declarations::enums::error_type_mismatch::wrong_type_in_generic_enum`
- `declarations::enums::error_type_mismatch::wrong_type_multiple_params`
- `declarations::enums::edge_cases::multiple_enums_same_case_names_different_scopes`
- `declarations::functions::basic::function_with_parameters`
- `execution_graph::enums::enum_struct_payloads::enum_with_struct_payload`
- `execution_graph::structs::field_access::four_level_nesting`
- `inference::array_inference::infer_array_element_type_mismatch`

**Root cause:** Regression in call resolution - either function calls with multiple parameters or initializer calls through type aliases are not being resolved correctly.

---

## Issue 3: Pattern Matching with `case` Keyword (Several tests)

Tests with enum pattern matching using `case` keyword in match arms are failing.

**Affected tests:**
- `builtins::boolean_conditional::in_if_statements::optional_in_if_condition`
- `builtins::boolean_conditional::with_result::result_as_boolean`
- `builtins::matchable::fallback_behavior::struct_destructuring_in_match`

**Root cause:** The tests use `case .Some(_)` syntax in match statements which appears broken.

---

## Issue 4 & 5: Test Framework Symbol Lookup Bug - FIXED

**THIS WAS A TEST FRAMEWORK BUG, NOT A COMPILER BUG**

The test framework's `find_by_name` did a depth-first search and found prelude's `ControlFlow[C, B]` type parameters before test module symbols.

**Fix:** Modified `lib/kestrel-test-suite/src/lib.rs` to use `SemanticModel.registry().find_by_kind_and_name()` for simple name lookups when a kind is specified.

**Tests now passing:**
- All `declarations::type_aliases::cycle_detection::*` tests (9 tests)
- Various other tests that used simple names like `B`, `C`

**Remaining failures in this category:** Tests that don't specify a kind still use tree search and may find wrong symbols (e.g., `function_with_params_and_attribute` finds prelude's `add` method).

---

## Issue 6: Function Attribute Parsing (1 test)

**Affected tests:**
- `attributes::declarations::function_declarations::function_with_params_and_attribute`

**Root cause:** Likely a parsing issue with attributes on functions with parameters.

---

## Issue 7: Literal Type Inference (1 test)

**Affected tests:**
- `builtins::literal_protocols::type_inference::literal_type_inferred_from_context`

**Root cause:** Literal type inference from function parameter context may be broken.

---

## Issue 8: Nil Literal Protocol (1 test)

**Affected tests:**
- `builtins::literal_protocols::expressible_by_nil_literal::optional_from_nil`

**Root cause:** ExpressibleByNilLiteral protocol handling may be broken.

---

## Summary of Root Causes

1. **Call resolution regression** - affecting function calls with multiple parameters and init calls
2. **Symbol resolution confusion** - type parameters being confused with protocols
3. **Type alias cycle detection** - false positives for valid chains
4. **Error message changes** - "type mismatch" → protocol conformance errors
5. **Pattern matching `case` syntax** - broken in match arms
