# Failing Tests Analysis

18 failing tests in `kestrel-test-suite`.

---

## Declarations (5 failures)

### 1. shorthand_missing_label
**Location:** `lib/kestrel-test-suite/tests/declarations/enums.rs:778`

**Issue:** Expects an error "no matching overload" when calling `.Circle(5.0)` without the parameter label `radius`, but the compiler isn't catching this.

---

### 2. extension_provides_associated_type_binding
**Location:** `lib/kestrel-test-suite/tests/declarations/extensions.rs:180`

**Issue:** Associated types in protocol extensions aren't being recognized - the extension needs to bind the associated type `Product = lang.i64`.

---

### 3. extension_conformance_on_generic_type
**Location:** `lib/kestrel-test-suite/tests/declarations/extensions.rs:232`

**Issue:** Generic type specialization in conformance (`extend Container[lang.i64]: Printable`) isn't working properly.

---

### 4. protocol_extension_calls_constraint_method
**Location:** `lib/kestrel-test-suite/tests/declarations/extensions.rs:1293`

**Issue:** Constrained protocol extensions (where clauses) aren't allowing calls to constraint protocol methods.

---

### 5. protocol_extension_calls_multiple_constraint_methods
**Location:** `lib/kestrel-test-suite/tests/declarations/extensions.rs:1323`

**Issue:** Multiple constraint protocols in protocol extensions aren't being resolved.

---

### 6. overload_by_parameter_type (Fixed)
**Location:** `lib/kestrel-test-suite/tests/declarations/functions.rs:105`

**Issue:** Conflicting expectation - the test expects both `.expect(Compiles)` AND `.expect(HasError("duplicate function signature"))`. Two functions with same name/different parameter types should either compile (overloading supported) OR fail (overloading not supported), not both.

---

## Execution Graph (3 failures)

### 7. closure_with_match
**Location:** `lib/kestrel-test-suite/tests/execution_graph/match_.rs:456`

**Issue:** Closures containing match expressions aren't compiling or aren't generating proper MIR.

---

### 8. init_with_parameter (Fixed)
**Location:** `lib/kestrel-test-suite/tests/execution_graph/structs.rs:537`

**Issue:** Initializer methods with parameters aren't being recognized properly - the parameter access mode isn't set to borrow by default.

---

### 9. calling_init (Fixed)
**Location:** `lib/kestrel-test-suite/tests/execution_graph/structs.rs:565`

**Issue:** Calling initializers with named parameters isn't working (`Counter(start: 42)`).

---

## Inference (5 failures)

### 10. static_method_in_extension_substitutes_type_param (Fixed)
**Location:** `lib/kestrel-test-suite/tests/inference/mod.rs:486`

**Issue:** Static methods in generic extensions aren't properly substituting type parameters when called with specializations like `Box[lang.i64].wrap(42)`.

---

### 11. static_method_in_extension_field_access (Fixed)
**Location:** `lib/kestrel-test-suite/tests/inference/mod.rs:512`

**Issue:** Static methods in generic extensions aren't properly substituting type parameters when called with specializations like `Box[lang.i64].wrap(42)`.

---

### 12. static_method_on_struct_substitutes_type_param (Fixed)
**Location:** `lib/kestrel-test-suite/tests/inference/mod.rs:537`

**Issue:** Static methods in generic extensions aren't properly substituting type parameters when called with specializations like `Box[lang.i64].wrap(42)`.

---

### 13. static_method_on_struct_field_access (Fixed)
**Location:** `lib/kestrel-test-suite/tests/inference/mod.rs:560`

**Issue:** Static methods in generic extensions aren't properly substituting type parameters when called with specializations like `Box[lang.i64].wrap(42)`.

---

### 14. static_method_infers_type_from_args (Fixed)
**Location:** `lib/kestrel-test-suite/tests/inference/mod.rs:583`

**Issue:** Static methods in generic extensions aren't properly substituting type parameters when called with specializations like `Box[lang.i64].wrap(42)`.

---

## Memory Model (3 failures)

### 15. assignment_of_copyable_uses_copy (Fixed)
**Location:** `lib/kestrel-test-suite/tests/memory_model/copy_semantics.rs:435`

**Issue:** MIR is using `move` instead of `copy` for assignments of copyable types.

---

### 16. basic_scope_exit_deinit
**Location:** `lib/kestrel-test-suite/tests/memory_model/deinit.rs:454`

**Issue:** Automatic deinit at scope exit isn't being inserted - the explicit `call Test.Handle.deinit()` is being generated instead of a `deinit` statement.

---

### 17. consuming_parameter_is_mutable (Fixed)
**Location:** `lib/kestrel-test-suite/tests/memory_model/parameter_access_modes.rs:502`

**Issue:** Test was using `*` operator on `lang.i64` instead of `lang.i64_mul()` intrinsic.

---

## Patterns (1 failure)

### 18. guard_must_be_bool (Fixed)
**Location:** `lib/kestrel-test-suite/tests/patterns/match_expressions.rs:475`

**Issue:** Guard condition type validation is missing - `n if n` (where `n` is `i64`) should fail but compiles successfully.

---

### 19. array_pattern_with_literals
**Location:** `lib/kestrel-test-suite/tests/patterns/pattern_types.rs:849`

**Issue:** Array patterns with literal values aren't supported in the parser/type checker.

---

## Types (1 failure)

### 20. type_param_shadowing_in_nested (Fixed)
**Location:** `lib/kestrel-test-suite/tests/types/generics.rs:1074`

**Issue:** Nested structs with shadowed type parameters aren't compiling.

---

### 21. type_args_on_primitive (Fixed)
**Location:** `lib/kestrel-test-suite/tests/types/generics.rs:684`

**Issue:** Type validation is missing - `lang.i64[lang.str]` should fail but compiles successfully.

---

### 22. lang_ptr_without_type_args_error (Fixed)
**Location:** `lib/kestrel-test-suite/tests/types/pointer.rs:108`

**Issue:** `lang.ptr` requires type arguments but validation is missing - `lang.ptr` alone should fail but compiles successfully.
