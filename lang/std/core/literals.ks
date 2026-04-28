// Literal protocols - types that can be constructed from literals
// These protocols enable types to be initialized from literal syntax.

module std.core

import std.numeric.(Int64, Float64)
import std.memory.(LiteralSlice)
import std.text.(String, Char)
import std.core.(Bool)

/// Protocol for types that accept a `true`/`false` literal.
///
/// The init takes a primitive `lang.i1` rather than `Bool` because `Bool`
/// itself conforms — the literal lowering needs a representation that does
/// not depend on the type being constructed.
@builtin(.ExpressibleByBoolLiteral)
public protocol ExpressibleByBoolLiteral {
    /// @name Bool Literal
    /// Builds an instance from a boolean literal.
    init(boolLiteral value: lang.i1)
}

/// Protocol for types that accept an integer literal (e.g. `42`, `0xff`).
///
/// All the standard integer widths conform; types outside `std.numeric` (for
/// example a `BigInt` or a fixed-point number) can also conform to opt in
/// to the literal syntax.
@builtin(.ExpressibleByIntLiteral)
public protocol ExpressibleByIntLiteral {
    /// @name Int Literal
    /// Builds an instance from an integer literal.
    init(intLiteral value: lang.i64)
}

/// Protocol for types that accept a floating-point literal (e.g. `3.14`).
@builtin(.ExpressibleByFloatLiteral)
public protocol ExpressibleByFloatLiteral {
    /// @name Float Literal
    /// Builds an instance from a floating-point literal.
    init(floatLiteral value: lang.f64)
}

/// Protocol for types that accept a string literal (`"…"`).
///
/// The init receives a primitive `lang.str` (pointer + length pair) so that
/// string literal lowering does not require the target type to already exist
/// in stdlib form.
@builtin(.ExpressibleByStringLiteral)
public protocol ExpressibleByStringLiteral {
    /// @name String Literal
    /// Builds an instance from a string literal.
    init(stringLiteral value: lang.str)
}

/// Protocol for types that accept a character literal (`'a'`).
@builtin(.ExpressibleByCharLiteral)
public protocol ExpressibleByCharLiteral {
    /// @name Char Literal
    /// Builds an instance from a character literal.
    init(charLiteral value: lang.i32)
}

/// Protocol for types that accept the `null` literal.
///
/// `Optional[T]` is the canonical conformer; it produces `.None`. Types
/// that wrap an optional or have a meaningful "absent" state may also
/// conform.
@builtin(.ExpressibleByNullLiteral)
public protocol ExpressibleByNullLiteral {
    /// @name Null Literal
    /// Builds the absent/none instance.
    init()
}

/// Compiler-internal protocol for array-literal lowering.
///
/// The lexer/parser lower `[a, b, c]` to a call into this init with a raw
/// pointer to a stack-allocated buffer of `Element`s. Only the compiler
/// uses this directly; user types should conform to
/// `ExpressibleByArrayLiteral` (which extends this with a friendlier API).
@builtin(._ExpressibleByArrayLiteral)
public protocol _ExpressibleByArrayLiteral {
    type Element

    /// @name Literal Bridge
    /// Compiler-emitted init taking a raw pointer and count.
    init(_arrayLiteralPointer _arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount _arrayLiteralCount: lang.i64)
}

/// User-facing protocol for array-literal lowering.
///
/// Provides a `LiteralSlice` view over the literal's contents so the
/// implementation can iterate or copy without juggling raw pointers.
public protocol ExpressibleByArrayLiteral: _ExpressibleByArrayLiteral {
    /// @name Array Literal
    /// Builds an instance from a literal slice of elements.
    init(arrayLiteral: LiteralSlice[Element])
}

// Bridge: default implementation satisfies _ExpressibleByArrayLiteral
//extend ExpressibleByArrayLiteral {
//    public init(_arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount: lang.i64) {
//        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount))
//    }
//}

/// Compiler-internal protocol for dictionary-literal lowering.
///
/// The compiler lowers `[k1: v1, k2: v2]` into a call with a raw pointer
/// to a `(Key, Value)` buffer. As with array literals, user types should
/// prefer `ExpressibleByDictionaryLiteral`.
@builtin(._ExpressibleByDictionaryLiteral)
public protocol _ExpressibleByDictionaryLiteral {
    type Key
    type Value

    /// @name Literal Bridge
    /// Compiler-emitted init taking a raw `(Key, Value)` pointer and count.
    init(_dictionaryLiteralPointer: lang.ptr[(Key, Value)], _dictionaryLiteralCount: lang.i64)
}

/// User-facing protocol for dictionary-literal lowering. Mirrors
/// `ExpressibleByArrayLiteral` but for key-value pairs.
public protocol ExpressibleByDictionaryLiteral: _ExpressibleByDictionaryLiteral {
    /// @name Dictionary Literal
    /// Builds an instance from a literal slice of key-value pairs.
    init(dictionaryLiteral: LiteralSlice[(Key, Value)])
}

// ============================================================================
// Default literal types
// ============================================================================
// Used when a literal's type cannot be inferred from context — e.g. `let x = 1`
// in isolation defaults `x` to `Int64`. Each alias is wired up via a `@builtin`
// tag so the type checker knows which type to pick when defaulting.

/// Default type for integer literals (`let x = 1` → `Int64`).
@builtin(.DefaultIntegerLiteralType)
public type IntegerLiteralType = Int64

/// Default type for float literals (`let x = 1.0` → `Float64`).
@builtin(.DefaultFloatLiteralType)
public type FloatLiteralType = Float64

/// Default type for string literals (`let s = "hi"` → `String`).
@builtin(.DefaultStringLiteralType)
public type StringLiteralType = String

/// Default type for boolean literals (`let b = true` → `Bool`).
@builtin(.DefaultBooleanLiteralType)
public type BooleanLiteralType = Bool

/// Default type for character literals (`let c = 'a'` → `Char`).
@builtin(.DefaultCharLiteralType)
public type CharLiteralType = Char

/// Default type for null literals (`let x = null` → `Optional[T]`).
@builtin(.DefaultNullLiteralType)
public type NullLiteralType[T] = std.result.Optional[T]

/// Default type for array literals (`let a = [1, 2, 3]` → `Array[Int64]`).
@builtin(.DefaultArrayLiteralType)
public type ArrayLiteralType[T] = std.collections.Array[T]

/// Default type for dictionary literals (`let d = ["a": 1]` →
/// `Dictionary[String, Int64, DefaultHasher]`).
@builtin(.DefaultDictionaryLiteralType)
public type DictionaryLiteralType[K, V] = std.collections.Dictionary[K, V, std.collections.DefaultHasher]
