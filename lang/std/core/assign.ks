// Compound assignment operator protocols
// These protocols enable operators like +=, -=, etc.

module std.core

/// Raw protocol backing the `+=` operator.
///
/// In-place mutation lets conforming types avoid the temporary that a
/// `self = self + other` rewrite would produce — important for collections
/// (e.g. `Array += other`) and other types where the binary `+` would copy.
@builtin(.AddAssignProtocol)
public protocol AddAssign[Rhs = Self] {
    /// Mutates `self` to `self + other`.
    @builtin(.AddAssignMethod)
    mutating func addAssign(other: Rhs)
}

/// Raw protocol backing the `-=` operator.
@builtin(.SubtractAssignProtocol)
public protocol SubtractAssign[Rhs = Self] {
    /// Mutates `self` to `self - other`.
    @builtin(.SubtractAssignMethod)
    mutating func subtractAssign(other: Rhs)
}

/// Raw protocol backing the `*=` operator.
@builtin(.MultiplyAssignProtocol)
public protocol MultiplyAssign[Rhs = Self] {
    /// Mutates `self` to `self * other`.
    @builtin(.MultiplyAssignMethod)
    mutating func multiplyAssign(other: Rhs)
}

/// Raw protocol backing the `/=` operator.
@builtin(.DivideAssignProtocol)
public protocol DivideAssign[Rhs = Self] {
    /// Mutates `self` to `self / other`.
    @builtin(.DivideAssignMethod)
    mutating func divideAssign(other: Rhs)
}

/// Raw protocol backing the `%=` operator.
@builtin(.ModuloAssignProtocol)
public protocol ModuloAssign[Rhs = Self] {
    /// Mutates `self` to `self % other`.
    @builtin(.ModuloAssignMethod)
    mutating func modAssign(other: Rhs)
}

/// Raw protocol backing the `&=` operator.
@builtin(.BitwiseAndAssignProtocol)
public protocol BitwiseAndAssign[Rhs = Self] {
    /// Mutates `self` to `self & other`.
    @builtin(.BitwiseAndAssignMethod)
    mutating func bitwiseAndAssign(other: Rhs)
}

/// Raw protocol backing the `|=` operator.
@builtin(.BitwiseOrAssignProtocol)
public protocol BitwiseOrAssign[Rhs = Self] {
    /// Mutates `self` to `self | other`.
    @builtin(.BitwiseOrAssignMethod)
    mutating func bitwiseOrAssign(other: Rhs)
}

/// Raw protocol backing the `^=` operator.
@builtin(.BitwiseXorAssignProtocol)
public protocol BitwiseXorAssign[Rhs = Self] {
    /// Mutates `self` to `self ^ other`.
    @builtin(.BitwiseXorAssignMethod)
    mutating func bitwiseXorAssign(other: Rhs)
}

/// Raw protocol backing the `<<=` operator.
@builtin(.ShiftLeftAssignProtocol)
public protocol LeftShiftAssign[Rhs] {
    /// Mutates `self` to `self << count`.
    @builtin(.ShiftLeftAssignMethod)
    mutating func shiftLeftAssign(by count: Rhs)
}

/// Raw protocol backing the `>>=` operator.
@builtin(.ShiftRightAssignProtocol)
public protocol RightShiftAssign[Rhs] {
    /// Mutates `self` to `self >> count`.
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
