// Literal protocols - types that can be constructed from literals

module std.core

import std.num.(Int64, Float64)
import std.memory.(LiteralSlice)
import std.text.(String, Char)
import std.core.(Bool)

@builtin(.ExpressibleByBoolLiteral)
public protocol ExpressibleByBoolLiteral {
    init(boolLiteral value: lang.i1)
}

@builtin(.ExpressibleByIntLiteral)
public protocol ExpressibleByIntLiteral {
    init(intLiteral value: lang.i64)
}

@builtin(.ExpressibleByFloatLiteral)
public protocol ExpressibleByFloatLiteral {
    init(floatLiteral value: lang.f64)
}

@builtin(.ExpressibleByStringLiteral)
public protocol ExpressibleByStringLiteral {
    init(stringLiteral value: lang.str)
}

@builtin(.ExpressibleByCharLiteral)
public protocol ExpressibleByCharLiteral {
    init(charLiteral value: lang.i32)
}

@builtin(.ExpressibleByNilLiteral)
public protocol ExpressibleByNilLiteral {
    init()
}

// Low-level protocol - compiler calls this directly with raw pointer + count
@builtin(._ExpressibleByArrayLiteral)
public protocol _ExpressibleByArrayLiteral {
    type Element
    init(_arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount: lang.i64)
}

// User-facing protocol - takes LiteralSlice for convenience
@builtin(.ExpressibleByArrayLiteral)
public protocol ExpressibleByArrayLiteral: _ExpressibleByArrayLiteral {
    // Element is inherited from _ExpressibleByArrayLiteral
    init(arrayLiteral: LiteralSlice[Element])
}

// Bridge: default implementation satisfies _ExpressibleByArrayLiteral
//extend ExpressibleByArrayLiteral {
//    public init(_arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount: lang.i64) {
//        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount))
//    }
//}

// Dictionary literal protocol
// @builtin(.ExpressibleByDictionaryLiteral)
// public protocol ExpressibleByDictionaryLiteral {
//     type Key
//     type Value
//     init(dictionaryLiteral pairs: [(Key, Value)])
// }

// Default literal types - used when literal type cannot be inferred from context
@builtin(.DefaultIntegerLiteralType)
public type IntegerLiteralType = Int64

@builtin(.DefaultFloatLiteralType)
public type FloatLiteralType = Float64

@builtin(.DefaultStringLiteralType)
public type StringLiteralType = String

@builtin(.DefaultBooleanLiteralType)
public type BooleanLiteralType = Bool

@builtin(.DefaultCharLiteralType)
public type CharLiteralType = Char

public type ArrayLiteralType[T] = std.collections.Array[T]
