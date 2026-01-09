// Literal protocols - types that can be constructed from literals

module std.ops

import std.core.(Int64, Float64)

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

@builtin(.ExpressibleByNilLiteral)
public protocol ExpressibleByNilLiteral {
    init()
}

@builtin(.ExpressibleByArrayLiteral)
public protocol ExpressibleByArrayLiteral {
    type Element
    init(arrayLiteral elements: [Element])
}

@builtin(.ExpressibleByDictionaryLiteral)
public protocol ExpressibleByDictionaryLiteral {
    type Key
    type Value
    init(dictionaryLiteral pairs: [(Key, Value)])
}

// Default literal types - used when literal type cannot be inferred from context
@builtin(.DefaultIntegerLiteralType)
public type IntegerLiteralType = Int64

@builtin(.DefaultFloatLiteralType)
public type FloatLiteralType = Float64
