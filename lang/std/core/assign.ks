// Compound assignment operator protocols
// These protocols enable operators like +=, -=, etc.

module std.core

/// Protocol for types that support addition assignment (+=).
@builtin(.AddAssignProtocol)
public protocol AddAssign[Rhs = Self] {
    /// Adds the other value to this one in place.
    @builtin(.AddAssignMethod)
    mutating func addAssign(other: Rhs)
}

/// Protocol for types that support subtraction assignment (-=).
@builtin(.SubtractAssignProtocol)
public protocol SubtractAssign[Rhs = Self] {
    /// Subtracts the other value from this one in place.
    @builtin(.SubtractAssignMethod)
    mutating func subtractAssign(other: Rhs)
}

/// Protocol for types that support multiplication assignment (*=).
@builtin(.MultiplyAssignProtocol)
public protocol MultiplyAssign[Rhs = Self] {
    /// Multiplies this value by the other in place.
    @builtin(.MultiplyAssignMethod)
    mutating func multiplyAssign(other: Rhs)
}

/// Protocol for types that support division assignment (/=).
@builtin(.DivideAssignProtocol)
public protocol DivideAssign[Rhs = Self] {
    /// Divides this value by the other in place.
    @builtin(.DivideAssignMethod)
    mutating func divideAssign(other: Rhs)
}

/// Protocol for types that support modulo assignment (%=).
@builtin(.ModuloAssignProtocol)
public protocol ModuloAssign[Rhs = Self] {
    /// Computes the remainder of this value divided by the other in place.
    @builtin(.ModuloAssignMethod)
    mutating func modAssign(other: Rhs)
}

/// Protocol for types that support bitwise AND assignment (&=).
@builtin(.BitwiseAndAssignProtocol)
public protocol BitwiseAndAssign[Rhs = Self] {
    /// Performs bitwise AND with the other value in place.
    @builtin(.BitwiseAndAssignMethod)
    mutating func bitwiseAndAssign(other: Rhs)
}

/// Protocol for types that support bitwise OR assignment (|=).
@builtin(.BitwiseOrAssignProtocol)
public protocol BitwiseOrAssign[Rhs = Self] {
    /// Performs bitwise OR with the other value in place.
    @builtin(.BitwiseOrAssignMethod)
    mutating func bitwiseOrAssign(other: Rhs)
}

/// Protocol for types that support bitwise XOR assignment (^=).
@builtin(.BitwiseXorAssignProtocol)
public protocol BitwiseXorAssign[Rhs = Self] {
    /// Performs bitwise XOR with the other value in place.
    @builtin(.BitwiseXorAssignMethod)
    mutating func bitwiseXorAssign(other: Rhs)
}

/// Protocol for types that support left shift assignment (<<=).
@builtin(.ShiftLeftAssignProtocol)
public protocol LeftShiftAssign[Rhs] {
    /// Shifts this value's bits left by the given count in place.
    @builtin(.ShiftLeftAssignMethod)
    mutating func shiftLeftAssign(by count: Rhs)
}

/// Protocol for types that support right shift assignment (>>=).
@builtin(.ShiftRightAssignProtocol)
public protocol RightShiftAssign[Rhs] {
    /// Shifts this value's bits right by the given count in place.
    @builtin(.ShiftRightAssignMethod)
    mutating func shiftRightAssign(by count: Rhs)
}

// TODO: Default implementations for types that implement the corresponding binary operator
// with Output = Self. These are commented out because the where clause syntax
// `where Protocol[Param].Output = Self` is not yet supported.
//
// extend Addable[Rhs]: AddAssign[Rhs] where Addable[Rhs].Output = Self {
//     public mutating func addAssign(other: Rhs) {
//         self = self.add(other)
//     }
// }
//
// ... etc for other protocols
