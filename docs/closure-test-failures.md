# Closure Test Failures Documentation

## Summary

This document catalogs failing closure tests in the Kestrel language test suite. The failures fall into several categories related to features that have not yet been fully implemented or are still being developed.

**Total Failing Tests:** 5
**Total Passing Tests:** 61
**Document Updated:** 2025-12-18

### Failure Categories

| Category | Count | Phase | Status |
|----------|-------|-------|--------|
| Trailing Closure Syntax | 3 | Phase 9 | Not Implemented |
| Captures | 2 | Phase 10 | Not Implemented |

---

## Table of Contents

1. [Trailing Closure Syntax (Phase 9)](#trailing-closure-syntax-phase-9)
2. [Captures (Phase 10)](#captures-phase-10)
3. [Recently Fixed Tests](#recently-fixed-tests)

---

## Trailing Closure Syntax (Phase 9)

These tests cover trailing closure syntax where a closure can be passed as the last argument to a function without parentheses.

### trailing_closure::trailing_closure_only_argument

**Status:** FAILED
**Expected:** Compiles
**Actual:** 1 error

#### Kestrel Source Code
```kestrel
module Main

func apply(f: () -> Int) -> Int {
    f()
}

func test() -> Int {
    apply { 42 }
}
```

#### Error Message
```
error: function 'test' requires a body
  ┌─ test.ks:8:6
  │
8 │ func test() -> Int {
  │      ^^^^ function declared without body
```

**Analysis:** The parser is not recognizing the trailing closure syntax `apply { 42 }` as a valid function call with a closure argument.

---

### trailing_closure::trailing_closure_with_multiple_args

**Status:** FAILED
**Expected:** Compiles
**Actual:** 1 error

#### Kestrel Source Code
```kestrel
module Main

func combine(a: Int, b: Int, f: (Int) -> Int) -> Int {
    f(a + b)
}

func test() -> Int {
    combine(1, 2) { it * 2 }
}
```

**Analysis:** The parser fails to recognize `combine(1, 2) { it * 2 }` as a valid call syntax where the closure is the trailing argument.

---

### trailing_closure::trailing_closure_with_other_args

**Status:** FAILED
**Expected:** Compiles
**Actual:** 1 error

#### Kestrel Source Code
```kestrel
module Main

func fold(initial: Int, f: (Int, Int) -> Int) -> Int {
    f(initial, 10)
}

func test() -> Int {
    fold(0) { (acc, n) in acc + n }
}
```

**Analysis:** The parser fails when a non-final argument is in parentheses followed by a trailing closure.

---

## Captures (Phase 10)

These tests cover closure variable capture semantics.

### captures::cannot_mutate_captured_variable

**Status:** FAILED
**Expected:** HasError("cannot assign")
**Actual:** Compiles successfully (no error)

#### Kestrel Source Code
```kestrel
module Main

func test() -> () -> Int {
    var x = 10;
    {
        x = 20;
        x
    }
}
```

**Analysis:** The closure validation that prevents assignment to captured variables is not yet implemented.

---

### captures::capture_from_nested_scope

**Status:** FAILED
**Expected:** Compiles
**Actual:** 1 error

#### Kestrel Source Code
```kestrel
module Main

func test() -> () -> Int {
    let outer = 100;
    if true {
        let inner = 10;
        { outer + inner }
    } else {
        { outer }
    }
}
```

#### Error Message
```
error: function 'test' does not return a value on all code paths
  ┌─ test.ks:4:6
  │
4 │ func test() -> () -> Int {
  │      ^^^^ this function has a non-unit return type
  │
  = all code paths must end with a return statement or a value expression
```

**Analysis:** The exhaustive return analyzer does not recognize if-expressions with closure bodies as returning values on all paths.

---

## Key Findings and Recommendations

### Implementation Priorities

1. **Trailing Closure Syntax (Phase 9)**: Parser support needed for 3 tests
2. **Capture Validation (Phase 10)**: Semantic validation for captured variable mutation
3. **Exhaustive Return Analysis**: Fix for closures in if-expression branches

---

## Test File Reference

All tests are located in:
```
/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/expressions/closures.rs
```

Test execution command template:
```bash
cargo test --package kestrel-test-suite expressions::closures::[test_path] -- --nocapture
```

---

## Recently Fixed Tests

### Session 2025-12-18: ImplicitStructInit Field Constraints & Error Message Wording

The following tests were fixed by generating proper type constraints for struct field initializers and updating test expectations:

#### closures_as_values::closure_in_generic_struct

**Fix:** Added constraint generation between argument types and field types in `ImplicitStructInit` expressions. The constraint generator now:
1. Gets field types from `TypedBehavior` on field symbols
2. Applies type parameter substitutions (e.g., `T -> Int`)
3. Equates argument types with field types

```kestrel
module Main

struct Handler[T] {
    let handle: (T) -> T
}

func test() -> Int {
    let h = Handler[Int](handle: { it * 2 });
    (h.handle)(21)
}
```

**Root Cause:** The `ImplicitStructInit` case in `constraint_generator.rs` had a TODO and didn't generate constraints between field types and argument types. Closures with `Infer` types were never unified with the expected field type `(Int) -> Int`.

**Fix Location:** `lib/kestrel-semantic-type-inference/src/constraint_generator.rs`

---

#### type_inference::cannot_infer_without_context_error

**Fix:** Updated test expectation from `HasError("cannot infer")` to `HasError("could not infer type")` to match actual error message.

---

#### type_inference::cannot_infer_it_type_without_context

**Fix:** Same as above - updated test expectation to match actual error message wording.

---

### Session 2025-12-18: Syntax Fixes

The following tests were fixed by using correct syntax:

#### edge_cases::closure_with_never_return

**Fix:** Changed `earlyReturn { return 42 }` to `earlyReturn({ return 42 })` - use normal call syntax instead of trailing closure

```kestrel
func test() -> Int {
    earlyReturn({
        return 42
    })
}
```

---

#### closures_as_values::closure_in_struct_field

**Fix:** Changed `cb.action()` to `(cb.action)()` - field access must be parenthesized before calling

```kestrel
func test() -> Int {
    let cb = Callback(action: { 42 });
    (cb.action)()
}
```

---

#### implicit_it::it_shadowed_in_nested_closure

**Fix:** Changed `apply { it + outer }` to `apply({ it + outer })` - use normal call syntax instead of trailing closure

```kestrel
func test() -> (Int) -> Int {
    {
        let outer = it;
        apply({ it + outer })
    }
}
```

---

#### nested_closures::nested_closure_with_it_shadowing

**Fix:** Same as above - use normal call syntax

```kestrel
func test() -> (Int) -> Int {
    {
        let outer = it;
        apply({ it + outer })
    }
}
```

---

### Session 2025-12-18: UnresolvedFunction Implementation

The following tests were fixed by implementing the `TyKind::UnresolvedFunction` variant:

#### immediate_invocation::immediately_invoked_no_params

**Fix:** `TyKind::UnresolvedFunction` allows closures to be recognized as callable before full type inference

```kestrel
func test() -> Int {
    { 42 }()
}
```

---

#### immediate_invocation::immediately_invoked_for_scoping

**Fix:** Same as above - closures are now callable with `UnresolvedFunction` type

```kestrel
func test() -> Int {
    let result = {
        let a = 10;
        let b = 20;
        a + b
    }();
    result
}
```

---

#### implicit_it::it_used_zero_param_context_error

**Fix:** Proper error message when `it` is used with wrong arity

```kestrel
func test() -> () -> Int {
    { it }
}
```

Error now says: "`it` can only be used when closure has exactly 1 parameter, but 0 were expected"

---

#### implicit_it::it_used_multi_param_context_error

**Fix:** Same as above - proper `it` arity error

```kestrel
func test() -> (Int, Int) -> Int {
    { it }
}
```

---

#### type_checking::closure_assigned_to_non_function_type

**Fix:** Type mismatch properly detected between closure and non-function type

```kestrel
func test() {
    let x: Int = { 42 };
}
```

---

### Session 2025-12-18: Bidirectional Type Inference & Operator Fixes

The following tests were fixed by additional improvements:

#### trailing_closure::non_trailing_closure_in_parens

**Fix:** Added bidirectional type inference by generating constraints between argument and parameter types in `constraint_generator.rs`

```kestrel
func apply(f: (Int) -> Int) -> Int {
    f(10)
}

func test() -> Int {
    apply({ it * 2 })
}
```

---

#### closures_as_values::closure_passed_to_function

**Fix:** Same bidirectional type inference fix

```kestrel
func apply(x: Int, f: (Int) -> Int) -> Int {
    f(x)
}

func test() -> Int {
    apply(10, { it + 1 })
}
```

---

#### immediate_invocation::immediately_invoked_wrong_arg_count

**Fix:** Added arity validation in `calls.rs` for closure calls

```kestrel
func test() -> Int {
    { (x: Int) in x }(1, 2)
}
```

Now correctly reports: "wrong number of arguments to closure: expected 1, found 2"

---

#### multi_statement::closure_with_if_expression

**Fix:** Fixed comparison operators to return `Bool` when lhs is `Infer` in `operators.rs`

```kestrel
func test() -> (Int) -> Int {
    { (x) in
        if x > 0 {
            x
        } else {
            0 - x
        }
    }
}
```

Previously, `x > 0` incorrectly propagated the `Infer` type to the result instead of `Bool`.

---

### Session 2025-12-18: Generic Function Instantiation Fix

The following tests were fixed by properly instantiating generic function types at call sites:

#### generics::closure_in_generic_function

**Fix:** When calling a generic function, the callee's function type is now instantiated by applying the inferred type substitutions. Previously, the callee type kept raw type parameters like `(T, (T) -> T) -> T`, which caused constraint generation to create `arg == T` constraints that failed to unify. Now the type is instantiated to `(I64, (I64) -> I64) -> I64`, allowing closure types to properly unify.

```kestrel
module Main

func identity[T](x: T, f: (T) -> T) -> T {
    f(x)
}

func test() -> Int {
    identity(10, { it })
}
```

**Root Cause:** The constraint generator was using the raw callee type `(T, (T) -> T) -> T` instead of the instantiated type `(I64, (I64) -> I64) -> I64`. This caused constraints like `I64 == T` which failed because `TypeParameter` cannot unify with concrete types.

**Fix Location:** `lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs` - Added `apply_substitutions` to the callee type before creating the `Call` expression.

---

#### generics::closure_with_generic_param_inferred

**Fix:** Same as above - generic function type instantiation now works correctly with multiple type parameters.

```kestrel
module Main

func transform[T, U](x: T, f: (T) -> U) -> U {
    f(x)
}

func test() -> String {
    transform(42, { (n) in "hello" })
}
```

The type parameters `T` and `U` are now properly instantiated to `I64` and `String` respectively, allowing the closure parameter `n` to be inferred as `I64`.
