// Literal protocols - types that can be constructed from literals
// These protocols enable types to be initialized from literal syntax.

module std.core

import std.num.(Int64, Float64)
import std.memory.(LiteralSlice)
import std.text.(String, Char)
import std.core.(Bool)

/// Protocol for types that can be constructed from boolean literals (true/false).
@builtin(.ExpressibleByBoolLiteral)
public protocol ExpressibleByBoolLiteral {
    /// Creates an instance from a boolean literal.
    init(boolLiteral value: lang.i1)
}

/// Protocol for types that can be constructed from integer literals.
@builtin(.ExpressibleByIntLiteral)
public protocol ExpressibleByIntLiteral {
    /// Creates an instance from an integer literal.
    init(intLiteral value: lang.i64)
}

/// Protocol for types that can be constructed from floating-point literals.
@builtin(.ExpressibleByFloatLiteral)
public protocol ExpressibleByFloatLiteral {
    /// Creates an instance from a floating-point literal.
    init(floatLiteral value: lang.f64)
}

/// Protocol for types that can be constructed from string literals.
@builtin(.ExpressibleByStringLiteral)
public protocol ExpressibleByStringLiteral {
    /// Creates an instance from a string literal.
    init(stringLiteral value: lang.str)
}

/// Protocol for types that can be constructed from character literals.
@builtin(.ExpressibleByCharLiteral)
public protocol ExpressibleByCharLiteral {
    /// Creates an instance from a character literal.
    init(charLiteral value: lang.i32)
}

/// Protocol for types that can be constructed from null literals.
@builtin(.ExpressibleByNullLiteral)
public protocol ExpressibleByNullLiteral {
    /// Creates a null/none instance.
    init()
}

/// Low-level protocol for array literal initialization.
/// The compiler calls this directly with raw pointer and count.
@builtin(._ExpressibleByArrayLiteral)
public protocol _ExpressibleByArrayLiteral {
    type Element
    /// Creates an instance from a raw pointer to elements and count.
    init(_arrayLiteralPointer _arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount _arrayLiteralCount: lang.i64)
}

/// User-facing protocol for array literal initialization.
/// Provides a more convenient interface using LiteralSlice.
/// The compiler only uses `_ExpressibleByArrayLiteral` directly; this is a
/// convenience wrapper a type may also adopt to take `LiteralSlice` arguments.
public protocol ExpressibleByArrayLiteral: _ExpressibleByArrayLiteral {
    /// Creates an instance from a literal slice of elements.
    init(arrayLiteral: LiteralSlice[Element])
}

// Bridge: default implementation satisfies _ExpressibleByArrayLiteral
//extend ExpressibleByArrayLiteral {
//    public init(_arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount: lang.i64) {
//        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount))
//    }
//}

/// Low-level protocol for dictionary literal initialization.
/// The compiler calls this directly with raw pointer and count.
@builtin(._ExpressibleByDictionaryLiteral)
public protocol _ExpressibleByDictionaryLiteral {
    type Key
    type Value
    /// Creates an instance from a raw pointer to key-value pairs and count.
    init(_dictionaryLiteralPointer: lang.ptr[(Key, Value)], _dictionaryLiteralCount: lang.i64)
}

/// User-facing protocol for dictionary literal initialization.
/// Provides a more convenient interface using LiteralSlice.
/// The compiler only uses `_ExpressibleByDictionaryLiteral` directly; this is
/// a convenience wrapper a type may also adopt to take `LiteralSlice` arguments.
public protocol ExpressibleByDictionaryLiteral: _ExpressibleByDictionaryLiteral {
    /// Creates an instance from a literal slice of key-value pairs.
    init(dictionaryLiteral: LiteralSlice[(Key, Value)])
}

// Default literal types - used when literal type cannot be inferred from context

/// The default type for integer literals when type cannot be inferred.
@builtin(.DefaultIntegerLiteralType)
public type IntegerLiteralType = Int64

/// The default type for floating-point literals when type cannot be inferred.
@builtin(.DefaultFloatLiteralType)
public type FloatLiteralType = Float64

/// The default type for string literals when type cannot be inferred.
@builtin(.DefaultStringLiteralType)
public type StringLiteralType = String

/// The default type for boolean literals when type cannot be inferred.
@builtin(.DefaultBooleanLiteralType)
public type BooleanLiteralType = Bool

/// The default type for character literals when type cannot be inferred.
@builtin(.DefaultCharLiteralType)
public type CharLiteralType = Char

/// The default type for null literals when type cannot be inferred.
@builtin(.DefaultNullLiteralType)
public type NullLiteralType[T] = std.result.Optional[T]

/// The default type for array literals when type cannot be inferred.
@builtin(.DefaultArrayLiteralType)
public type ArrayLiteralType[T] = std.collections.Array[T]

/// The default type for dictionary literals when type cannot be inferred.
@builtin(.DefaultDictionaryLiteralType)
public type DictionaryLiteralType[K, V] = std.collections.Dictionary[K, V, std.collections.DefaultHasher]
