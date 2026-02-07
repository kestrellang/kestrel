# Optional and Result Type Promotion Design

## Overview

This feature implements implicit type promotion for Optional and Result types in Kestrel using a **closed protocol-based approach**. Values are automatically wrapped via the internal `FromValue` protocol, providing perfect symmetry with the existing `FromResidual` protocol used by `throw`.

### Motivation

Type promotion reduces boilerplate when working with Optional and Result types, making code more ergonomic. This completes the abstraction where `T throws E` lets users think in terms of returning `T` and throwing `E`, not manually constructing `Result.Ok` and `Result.Err`.

```kestrel
// Without promotion:
let x: Int? = Optional.Some(5)
fn get() -> Int throws Error { return Result.Ok(42) }
throw error  // Already works: desugars to return R.fromResidual(error)

// With promotion:
let x: Int? = 5
fn get() -> Int throws Error { return 42 }  // Desugars to return R.from(value)
throw error  // Desugars to return R.fromResidual(error)
```

### Design Philosophy: Symmetry with FromResidual

Kestrel already has `FromResidual[Early]` for error propagation:
- `throw error` desugars to `return R.fromResidual(error)`

This feature adds `FromValue[Output]` for success propagation:
- `return value` (in Result-returning function) desugars to `return R.from(value)`

This creates a beautiful symmetry where both branches of error handling use the same protocol pattern:

| Operation | Protocol | Method | Direction |
|-----------|----------|--------|-----------|
| `throw error` | `FromResidual[Early]` | `fromResidual(_:)` | Error → Container |
| `return value` | `FromValue[Output]` | `from(_:)` | Value → Container |

## Syntax

No new syntax is added. Existing type syntax works with implicit promotion:

```kestrel
// Optional types
let x: Int? = 5                    // Promoted to Optional.Some(5)
let y: String? = "hello"           // Promoted to Optional.Some("hello")

// Result types  
let r: Int throws Error = 42       // Promoted to Result.Ok(42)

// In function returns
fn getValue() -> Int? {
    return 5                       // Promoted to Optional.Some(5)
}

fn compute() -> Int throws Error {
    return 42                      // Promoted to Result.Ok(42)
}

// In assignments
var opt: Int? = 0
opt = 10                           // Promoted to Optional.Some(10)
```

## The FromValue Protocol (Internal)

```kestrel
/// Protocol for types that can be constructed from a success/output value.
/// Enables implicit promotion: `let opt: Int? = 5` desugars to `Optional.from(5)`
/// 
/// INTERNAL: This is a closed protocol used only by Optional and Result.
/// It is not public and cannot be implemented by user-defined types.
@builtin(.FromValueProtocol)
protocol FromValue[Output] {
    /// Creates an instance from a success value.
    @builtin(.FromValueMethod)
    static func from(_ value: Output) -> Self
}
```

### Standard Library Conformances (Closed)

```kestrel
// Optional conforms to FromValue (internal implementation detail)
extend Optional[T]: FromValue[T] {
    static func from(_ value: T) -> Optional[T] {
        .Some(value)
    }
}

// Result conforms to FromValue (internal implementation detail)
extend Result[T, E]: FromValue[T] {
    static func from(_ value: T) -> Result[T, E] {
        .Ok(value)
    }
}
```

**Note**: `FromValue` is internal to the standard library. Only `Optional` and `Result` implement it. This is a closed language feature, not an extensible user protocol.

## Semantic Behavior

### Promotion via FromValue Conformance

Type promotion works by checking if the **target type conforms to `FromValue[From]`**, where `From` is the value's type:

1. **Type Checking Phase**: When checking if `From` is assignable to `To`:
   - First check normal assignability (`From.is_assignable_to(To)`)
   - If that fails, check if `To` conforms to `FromValue[From]`
   - If conformance exists, the assignment is valid with promotion

2. **Lowering Phase**: The expression is desugared to a static method call:
   - `value` → `TargetType.from(value)`
   - This mirrors how `throw error` desugars to `return R.fromResidual(error)`

### Optional Promotion

```kestrel
let x: Int? = 5
```

1. Type check: Does `Int?` (which expands to `Optional[Int]`) conform to `FromValue[Int]`?
2. Yes! Optional has `extend Optional[T]: FromValue[T]`
3. Lowering: Desugar to `Optional.from(5)` which returns `Optional.Some(5)`

### Result Promotion

```kestrel
let r: Int throws Error = 42
```

1. Type check: Does `Int throws Error` (which expands to `Result[Int, Error]`) conform to `FromValue[Int]`?
2. Yes! Result has `extend Result[T, E]: FromValue[T]`
3. Lowering: Desugar to `Result.from(42)` which returns `Result.Ok(42)`

### Return Statement Promotion

In functions with Optional or Result return types, `return value` automatically promotes:

```kestrel
fn getValue() -> Int? {
    return 5  // Desugars to: return Optional.from(5)
}

fn compute() -> Int throws Error {
    return 42  // Desugars to: return Result.from(42)
}
```

This is the **primary use case** and provides symmetry with `throw`:

```kestrel
fn risky() -> Int throws Error {
    if someCondition {
        throw MyError.Failed     // Desugars to: return Result.fromResidual(MyError.Failed)
    }
    return 42                   // Desugars to: return Result.from(42)
}
```

### Promotion Contexts

Promotion applies in these contexts:

1. **Variable declarations**: `let x: Int? = 5`
2. **Variable assignments**: `x = 10` where `x: Int?`
3. **Return statements**: `return value` where function returns `T?` or `T throws E`
4. **If expression branches**: When branches have Optional/Result type
5. **Function arguments**: When parameter is Optional/Result type

### Single-Level Promotion

Promotion is single-level only. This means:

```kestrel
let x: Int? = 5          // OK: 5 -> Optional.Some(5)
let y: Int?? = 5         // ERROR: 5 is Int, not Int?
let z: Int?? = x         // OK if x is Int? - x is already Optional
```

Multi-level promotion would be confusing and error-prone.

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| Value type incompatible with inner type | "Cannot assign `{found}` to `{expected}`: type mismatch" |
| Attempting multi-level promotion | "Cannot promote `{found}` to `{expected}`: expected `{inner}`" |
| Promotion in non-promotion context | No error - no promotion attempted |

## Edge Cases

### Type Inference

When the target type is inferred rather than explicit, promotion does NOT occur:

```kestrel
let x = 5              // x is Int, not Int?
let opt: Int? = x      // opt is Int?, assigned from Int (promoted via FromValue[Int])
```

### Generic Contexts

With type parameters, promotion works via protocol conformance:

```kestrel
fn wrap[T](value: T) -> T? {
    return value       // OK: Optional[T] conforms to FromValue[T]
}
```

### Nested Optionals

Only single-level promotion is supported. The protocol conformance check prevents multi-level:

```kestrel
let a: Int? = 5        // OK: Optional[Int] conforms to FromValue[Int]
let b: Int?? = a       // OK: a is already Int?
let c: Int?? = 5       // ERROR: Optional[Optional[Int]] does NOT conform to FromValue[Int]
                        // (it conforms to FromValue[Optional[Int]] instead)
```

### Never Type

Never type values are assignable to any type, including Optional/Result, without promotion (they diverge):

```kestrel
fn fail() -> Never { loop { } }
let x: Int? = fail()   // OK: Never is assignable to anything (no FromValue check needed)
```

### Error Type

Error type propagates through promotion contexts without triggering promotion:

```kestrel
let x: Int? = someError   // Error type - no promotion attempted
```

### Closed Protocol Restriction

`FromValue` is internal and cannot be implemented by user types. This is intentional - promotion is a language feature for Optional and Result only:

```kestrel
// ERROR: FromValue is not public
extend MyResult[T, E]: FromValue[T] { ... }
```

If users want similar behavior, they should use explicit constructors or the `Convertible` protocol.

## Open Questions (Resolved)

1. **Q: Should promotion work for function arguments?**
   A: **Yes**. Since Kestrel has no type-based overloading, function argument promotion is safe. Check `FromValue` conformance.

2. **Q: Should promotion work in match arms?**
   A: Yes, in all contexts where the target type is known.

3. **Q: How to detect Optional/Result types reliably?**
   A: **FromValue conformance!** Check if target type conforms to `FromValue[From]` using the type system's conformance checking.

4. **Q: Should nil literal get promotion?**
   A: No, nil desugars directly to `Optional.None` during parsing/binding.

5. **Q: What about throw expressions?**
   A: `throw error` already uses `FromResidual`. `return value` in Result functions uses `FromValue`. Perfect symmetry.

6. **Q: Protocol naming?**
   A: **Decision: Use `FromValue[Output]`** to mirror `FromResidual[Early]`:
   - `FromResidual[Early]` with `fromResidual(_:)` for error propagation
   - `FromValue[Output]` with `from(_:)` for success propagation
   - Both are closed internal protocols

7. **Q: Should FromValue be public?**
   A: **No**. `FromValue` is internal to stdlib. Only Optional and Result implement it. This is a closed language feature.

8. **Q: What about the existing Returnable protocol?**
   A: **Remove it entirely**. It was unused and overlaps with this feature. (Already done)

## Implementation Notes

The implementation has three parts:

### 1. Standard Library Changes
- Add `@builtin(.FromValueProtocol)` and `@builtin(.FromValueMethod)` to `std/core/error.ks` (alongside `FromResidual`)
- Add `FromValue[T]` conformance to `Optional` in `std/result/optional.ks`
- Add `FromValue[T]` conformance to `Result` in `std/result/result.ks`
- Remove unused `Returnable` protocol from `std/core/error.ks`

### 2. Type Checking
Modify `is_assignable_with_promotion()` to check `FromValue` conformance:
- First try normal assignability
- If that fails, check if target conforms to `FromValue[From]`
- Update all type check locations (variable bindings, returns, assignments, arguments)

Files:
- `lib/kestrel-semantic-tree/src/builtins.rs` - Add `FromValueProtocol`, `FromValueMethod`
- `lib/kestrel-semantic-analyzers/src/analyzers/type_assignability/mod.rs` - Add promotion check
- `lib/kestrel-semantic-analyzers/src/analyzers/type_check/mod.rs` - Use promotion-aware assignability

### 3. Lowering
Desugar promoted expressions to static method calls:
- `value` → `TargetType.from(value)`
- Similar to how `throw error` desugars to `return R.fromResidual(error)`

Files:
- `lib/kestrel-execution-graph-lowering/src/expr.rs` - Add promotion lowering
- Create deferred static call to `from` method

### Perfect Symmetry with FromResidual

| Aspect | FromResidual (for throw) | FromValue (for return) |
|--------|-------------------------|------------------------|
| **Use case** | Error propagation | Success propagation |
| **Protocol** | `FromResidual[Early]` | `FromValue[Output]` |
| **Method** | `fromResidual(_:)` | `from(_:)` |
| **Trigger** | `throw error` | `return value` (in Optional/Result function) |
| **Direction** | Error → Container | Value → Container |
| **Visibility** | Internal | Internal |
| **Lowering** | `return R.fromResidual(error)` | `return R.from(value)` |

This completes the abstraction where `T throws E` lets users think in terms of returning `T` and throwing `E`, not manually constructing `Result.Ok` and `Result.Err`.
