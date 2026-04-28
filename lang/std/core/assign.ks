// Compound assignment operator protocols
// These protocols enable operators like +=, -=, etc.

module std.core

/// Raw protocol backing the `+=` operator.
///
/// In-place mutation lets conforming types avoid the temporary that a
/// `self = self + other` rewrite would produce — important for collections
/// (e.g. `Array += other`) and other types where the binary `+` would copy.
@builtin(.AddAssignProtocol)
public protocol AddAssign[Other = Self] {
    /// Mutates `self` to `self + other`.
    @builtin(.AddAssignMethod)
    mutating func addAssign(other: Other)
}

/// Raw protocol backing the `-=` operator.
@builtin(.SubtractAssignProtocol)
public protocol SubtractAssign[Other = Self] {
    /// Mutates `self` to `self - other`.
    @builtin(.SubtractAssignMethod)
    mutating func subtractAssign(other: Other)
}

/// Raw protocol backing the `*=` operator.
@builtin(.MultiplyAssignProtocol)
public protocol MultiplyAssign[Other = Self] {
    /// Mutates `self` to `self * other`.
    @builtin(.MultiplyAssignMethod)
    mutating func multiplyAssign(other: Other)
}

/// Raw protocol backing the `/=` operator.
@builtin(.DivideAssignProtocol)
public protocol DivideAssign[Other = Self] {
    /// Mutates `self` to `self / other`.
    @builtin(.DivideAssignMethod)
    mutating func divideAssign(other: Other)
}

/// Raw protocol backing the `%=` operator.
@builtin(.ModuloAssignProtocol)
public protocol ModuloAssign[Other = Self] {
    /// Mutates `self` to `self % other`.
    @builtin(.ModuloAssignMethod)
    mutating func modAssign(other: Other)
}

/// Raw protocol backing the `&=` operator.
@builtin(.BitwiseAndAssignProtocol)
public protocol BitwiseAndAssign[Other = Self] {
    /// Mutates `self` to `self & other`.
    @builtin(.BitwiseAndAssignMethod)
    mutating func bitwiseAndAssign(other: Other)
}

/// Raw protocol backing the `|=` operator.
@builtin(.BitwiseOrAssignProtocol)
public protocol BitwiseOrAssign[Other = Self] {
    /// Mutates `self` to `self | other`.
    @builtin(.BitwiseOrAssignMethod)
    mutating func bitwiseOrAssign(other: Other)
}

/// Raw protocol backing the `^=` operator.
@builtin(.BitwiseXorAssignProtocol)
public protocol BitwiseXorAssign[Other = Self] {
    /// Mutates `self` to `self ^ other`.
    @builtin(.BitwiseXorAssignMethod)
    mutating func bitwiseXorAssign(other: Other)
}

/// Raw protocol backing the `<<=` operator.
@builtin(.ShiftLeftAssignProtocol)
public protocol LeftShiftAssign[Other] {
    /// Mutates `self` to `self << count`.
    @builtin(.ShiftLeftAssignMethod)
    mutating func shiftLeftAssign(by count: Other)
}

/// Raw protocol backing the `>>=` operator.
@builtin(.ShiftRightAssignProtocol)
public protocol RightShiftAssign[Other] {
    /// Mutates `self` to `self >> count`.
    @builtin(.ShiftRightAssignMethod)
    mutating func shiftRightAssign(by count: Other)
}

// TODO: Default implementations for types that implement the corresponding binary operator
// with Output = Self. These are commented out because the where clause syntax
// `where Protocol[Param].Output = Self` is not yet supported.
//
// extend Addable[Other]: AddAssign[Other] where Addable[Other].Output = Self {
//     public mutating func addAssign(other: Other) {
//         self = self.add(other)
//     }
// }
//
// ... etc for other protocols
