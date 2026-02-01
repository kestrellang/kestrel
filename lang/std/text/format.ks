// String Interpolation - Protocols and types for string interpolation.
// Note: The actual interpolation support requires compiler changes.
// For now, this file defines the protocols that will be used.

module std.text

import std.core.(Bool, Formattable, ExpressibleByStringLiteral)
import std.core.(FormatOptions)
import std.num.(Int64)
import std.text.(String)
import std.collections.(Array)

// ============================================================================
// STRING INTERPOLATION PROTOCOLS
// ============================================================================

/// Protocol for types that accumulate string interpolation parts.
public protocol StringInterpolationProtocol {
    /// Initialize with capacity hints for optimization.
    init(literalCapacity: Int64, interpolationCount: Int64)

    /// Append a literal string segment.
    mutating func appendLiteral(literal: String)

    /// Append a formatted value with options.
    mutating func appendInterpolation[T](value: T, options: FormatOptions) where T: Formattable
}

/// Protocol for types that can be created from string interpolation.
public protocol ExpressibleByStringInterpolation: ExpressibleByStringLiteral {
    /// The type used to accumulate interpolation parts.
    type Interpolation: StringInterpolationProtocol

    /// Create from a completed interpolation.
    init(interpolation: Interpolation)
}

// ============================================================================
// DEFAULT STRING INTERPOLATION
// ============================================================================

/// Default implementation of StringInterpolationProtocol for String.
@builtin(.DefaultStringInterpolation)
public struct DefaultStringInterpolation: StringInterpolationProtocol {
    private var parts: Array[String]

    /// Initialize with capacity hints.
    @builtin(.DefaultStringInterpolationInit)
    public init(literalCapacity: Int64, interpolationCount: Int64) {
        self.parts = [];
    }

    /// Append a literal string segment.
    @builtin(.DefaultStringInterpolationAppendLiteral)
    public mutating func appendLiteral(literal: String) {
        if literal.isEmpty == false {
            self.parts.append(literal);
        }
    }

    /// Append a formatted value with options.
    @builtin(.DefaultStringInterpolationAppendInterpolation)
    public mutating func appendInterpolation[T](value: T, options: FormatOptions = FormatOptions.default()) where T: Formattable {
        self.parts.append(value.format());
    }

    /// Build the final string by concatenating all parts.
    @builtin(.DefaultStringInterpolationBuild)
    public func build() -> String {
        let partsCount = self.parts.count;
        if partsCount == 0 {
            return ""
        }
        var result = "";
        var i: Int64 = 0;
        while i < partsCount {
            let part = self.parts.getValue(at: i).unwrap();
            result = result + part;
            i = i + 1;
        }
        result
    }
}
