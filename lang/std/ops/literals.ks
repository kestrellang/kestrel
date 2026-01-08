// Literal protocols - types that can be constructed from literals

@builtin(.ExpressibleByBoolLiteral)
public protocol ExpressibleByBoolLiteral {
    init(boolLiteral value: Bool)
}

@builtin(.ExpressibleByIntLiteral)
public protocol ExpressibleByIntLiteral {
    init(intLiteral value: Int)
}

@builtin(.ExpressibleByFloatLiteral)
public protocol ExpressibleByFloatLiteral {
    init(floatLiteral value: Float64)
}

@builtin(.ExpressibleByStringLiteral)
public protocol ExpressibleByStringLiteral {
    init(stringLiteral value: String)
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
