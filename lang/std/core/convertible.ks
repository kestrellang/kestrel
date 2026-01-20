// Type conversion protocol

module std.core

// Convertible - for type conversions
public protocol Convertible[From] {
    init(from value: From)
}
