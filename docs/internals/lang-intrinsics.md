# Lang Intrinsics Reference

The `lang` module provides compiler-intrinsic types and functions that map directly to low-level operations. These are the primitive building blocks used by the standard library to implement higher-level types like `Int`, `Bool`, and `String`.

> **Note:** Lang intrinsics cannot use operators directly. Use the intrinsic functions (e.g., `lang.i64_add(a, b)` instead of `a + b`).

## Primitive Types

### Integer Types

| Type | Description | Bits |
|------|-------------|------|
| `lang.i1` | Boolean (1-bit integer) | 1 |
| `lang.i8` | 8-bit signed integer | 8 |
| `lang.i16` | 16-bit signed integer | 16 |
| `lang.i32` | 32-bit signed integer | 32 |
| `lang.i64` | 64-bit signed integer | 64 |

> **Note:** There are no separate unsigned integer types at the lang level. Signedness is determined by the operation used (e.g., `signed_div` vs `unsigned_div`).

### Float Types

| Type | Description | Bits |
|------|-------------|------|
| `lang.f16` | 16-bit floating point (half precision) | 16 |
| `lang.f32` | 32-bit floating point (single precision) | 32 |
| `lang.f64` | 64-bit floating point (double precision) | 64 |

### Other Types

| Type | Description |
|------|-------------|
| `lang.str` | String reference (pointer + length) |
| `lang.ptr[T]` | Raw pointer to type T |

## Integer Operations

Integer intrinsics follow the pattern `lang.<type>_<op>(args...)` where `<type>` is one of `i1`, `i8`, `i16`, `i32`, `i64`.

### Signedness-Agnostic Operations

These operations produce the same result regardless of whether the integers are interpreted as signed or unsigned.

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.i*_add(a, b)` | Addition | 2 |
| `lang.i*_sub(a, b)` | Subtraction | 2 |
| `lang.i*_mul(a, b)` | Multiplication | 2 |
| `lang.i*_eq(a, b)` | Equality (returns `lang.i1`) | 2 |
| `lang.i*_ne(a, b)` | Inequality (returns `lang.i1`) | 2 |
| `lang.i*_and(a, b)` | Bitwise AND | 2 |
| `lang.i*_or(a, b)` | Bitwise OR | 2 |
| `lang.i*_xor(a, b)` | Bitwise XOR | 2 |
| `lang.i*_shl(a, b)` | Left shift | 2 |
| `lang.i*_neg(a)` | Negation (two's complement) | 1 |
| `lang.i*_not(a)` | Bitwise NOT | 1 |

### Signed Operations

Use these when treating integers as signed values.

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.i*_signed_div(a, b)` | Signed division | 2 |
| `lang.i*_signed_rem(a, b)` | Signed remainder | 2 |
| `lang.i*_signed_shr(a, b)` | Arithmetic right shift (sign-extending) | 2 |
| `lang.i*_signed_lt(a, b)` | Signed less than (returns `lang.i1`) | 2 |
| `lang.i*_signed_le(a, b)` | Signed less than or equal (returns `lang.i1`) | 2 |
| `lang.i*_signed_gt(a, b)` | Signed greater than (returns `lang.i1`) | 2 |
| `lang.i*_signed_ge(a, b)` | Signed greater than or equal (returns `lang.i1`) | 2 |

### Unsigned Operations

Use these when treating integers as unsigned values.

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.i*_unsigned_div(a, b)` | Unsigned division | 2 |
| `lang.i*_unsigned_rem(a, b)` | Unsigned remainder | 2 |
| `lang.i*_unsigned_shr(a, b)` | Logical right shift (zero-extending) | 2 |
| `lang.i*_unsigned_lt(a, b)` | Unsigned less than (returns `lang.i1`) | 2 |
| `lang.i*_unsigned_le(a, b)` | Unsigned less than or equal (returns `lang.i1`) | 2 |
| `lang.i*_unsigned_gt(a, b)` | Unsigned greater than (returns `lang.i1`) | 2 |
| `lang.i*_unsigned_ge(a, b)` | Unsigned greater than or equal (returns `lang.i1`) | 2 |

## Boolean (i1) Operations

The `lang.i1` type represents booleans. These dedicated operations are provided for clarity:

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.i1_eq(a, b)` | Boolean equality | 2 |
| `lang.i1_and(a, b)` | Boolean AND | 2 |
| `lang.i1_or(a, b)` | Boolean OR | 2 |
| `lang.i1_not(a)` | Boolean NOT | 1 |

## Float Operations

Float intrinsics follow the pattern `lang.<type>_<op>(args...)` where `<type>` is one of `f16`, `f32`, `f64`.

### Arithmetic Operations

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.f*_add(a, b)` | Addition | 2 |
| `lang.f*_sub(a, b)` | Subtraction | 2 |
| `lang.f*_mul(a, b)` | Multiplication | 2 |
| `lang.f*_div(a, b)` | Division | 2 |
| `lang.f*_neg(a)` | Negation | 1 |

### Comparison Operations

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.f*_eq(a, b)` | Equality (returns `lang.i1`) | 2 |
| `lang.f*_ne(a, b)` | Inequality (returns `lang.i1`) | 2 |
| `lang.f*_lt(a, b)` | Less than (returns `lang.i1`) | 2 |
| `lang.f*_le(a, b)` | Less than or equal (returns `lang.i1`) | 2 |
| `lang.f*_gt(a, b)` | Greater than (returns `lang.i1`) | 2 |
| `lang.f*_ge(a, b)` | Greater than or equal (returns `lang.i1`) | 2 |

### Math Operations

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.f*_floor(x)` | Floor (round toward negative infinity) | 1 |
| `lang.f*_ceil(x)` | Ceiling (round toward positive infinity) | 1 |
| `lang.f*_round(x)` | Round to nearest integer | 1 |
| `lang.f*_trunc(x)` | Truncate (round toward zero) | 1 |
| `lang.f*_sqrt(x)` | Square root | 1 |

### Predicates

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.f*_is_nan(x)` | Check if NaN (returns `lang.i1`) | 1 |
| `lang.f*_is_infinite(x)` | Check if infinite (returns `lang.i1`) | 1 |

### Constants

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.f*_infinity()` | Positive infinity | 0 |
| `lang.f*_nan()` | NaN (Not a Number) | 0 |

## Type Casts

Convert between primitive types using `lang.cast_<from>_<to>(value)`.

```
lang.cast_i8_i16(x)    // i8 -> i16 (sign-extend)
lang.cast_i32_i64(x)   // i32 -> i64 (sign-extend)
lang.cast_i64_i32(x)   // i64 -> i32 (truncate)
lang.cast_i64_f64(x)   // i64 -> f64 (int to float)
lang.cast_f64_i64(x)   // f64 -> i64 (float to int)
lang.cast_f32_f64(x)   // f32 -> f64 (float widen)
lang.cast_f64_f32(x)   // f64 -> f32 (float narrow)
```

All combinations of `i1`, `i8`, `i16`, `i32`, `i64`, `f16`, `f32`, `f64` are supported.

## Pointer Operations

Raw pointer manipulation for low-level memory operations.

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.ptr_null[T]()` | Create null pointer of type `T` | 0 |
| `lang.ptr_from_address[T](addr)` | Create pointer from integer address | 1 |
| `lang.ptr_to_address(ptr)` | Get integer address from pointer | 1 |
| `lang.ptr_to[T](value)` | Create pointer to value (stack allocation) | 1 |
| `lang.ptr_read[T](ptr)` | Dereference pointer (read value) | 1 |
| `lang.ptr_write[T](ptr, value)` | Write value through pointer | 2 |
| `lang.ptr_offset(ptr, offset)` | Offset pointer by bytes | 2 |
| `lang.ptr_is_null(ptr)` | Check if pointer is null (returns `lang.i1`) | 1 |
| `lang.cast_ptr[T](ptr)` | Cast pointer to different pointee type | 1 |

## Size and Alignment

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.sizeof[T]()` | Size of type `T` in bytes | 0 |
| `lang.alignof[T]()` | Alignment of type `T` in bytes | 0 |

## Atomic Operations

Atomic memory operations for concurrent programming.

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.atomic_add(place, delta)` | Atomic fetch-add (returns old value) | 2 |
| `lang.atomic_sub(place, delta)` | Atomic fetch-sub (returns old value) | 2 |

## Special Functions

| Function | Description | Arity |
|----------|-------------|-------|
| `lang.panic_unwind(message)` | Terminate program with panic message (returns `Never`) | 1 |

## Usage Examples

### Stdlib Bool Implementation

```kestrel
public struct Bool {
    private var value: lang.i1

    public init(boolLiteral value: lang.i1) {
        self.value = value
    }

    public func equals(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_eq(self.value, other.value))
    }

    public func logicalAnd(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_and(self.value, other.value))
    }

    public func logicalNot() -> Bool {
        Bool(boolLiteral: lang.i1_not(self.value))
    }
}
```

### Stdlib Int64 Implementation

```kestrel
public struct Int64 {
    public var raw: lang.i64

    public init(intLiteral value: lang.i64) {
        self.raw = value
    }

    public func add(other: Int64) -> Int64 {
        Int64(raw: lang.i64_add(self.raw, other.raw))
    }

    public func compare(other: Int64) -> Ordering {
        if Bool(boolLiteral: lang.i64_signed_lt(self.raw, other.raw)) { .Less }
        else if Bool(boolLiteral: lang.i64_signed_gt(self.raw, other.raw)) { .Greater }
        else { .Equal }
    }

    // Type conversion using casts
    public init(from other: Int32) {
        self.raw = lang.cast_i32_i64(other.raw)
    }
}
```

## Design Notes

1. **No Operator Overloading on Lang Types**: Lang intrinsic types cannot use operators (`+`, `-`, `<`, etc.). This is intentional - operators are implemented at the stdlib level through protocols like `Addable`, which call lang intrinsics internally.

2. **Signedness via Operations**: Unlike some languages that have separate signed/unsigned types, Kestrel uses a single integer type per bit width. Signedness is determined by which operation you use (`signed_div` vs `unsigned_div`).

3. **Wrapping Semantics**: Integer overflow wraps (two's complement). There are no checked arithmetic intrinsics at the lang level.

4. **IEEE 754 Floats**: Float operations follow IEEE 754 semantics, including NaN propagation and signed zeros.
