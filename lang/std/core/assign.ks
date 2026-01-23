// Compound assignment operator protocols

module std.core

@builtin(.AddAssignProtocol)
public protocol AddAssign[Rhs = Self] {
    @builtin(.AddAssignMethod)
    mutating func addAssign(other: Rhs)
}

@builtin(.SubtractAssignProtocol)
public protocol SubtractAssign[Rhs = Self] {
    @builtin(.SubtractAssignMethod)
    mutating func subtractAssign(other: Rhs)
}

@builtin(.MultiplyAssignProtocol)
public protocol MultiplyAssign[Rhs = Self] {
    @builtin(.MultiplyAssignMethod)
    mutating func multiplyAssign(other: Rhs)
}

@builtin(.DivideAssignProtocol)
public protocol DivideAssign[Rhs = Self] {
    @builtin(.DivideAssignMethod)
    mutating func divideAssign(other: Rhs)
}

@builtin(.ModuloAssignProtocol)
public protocol ModuloAssign[Rhs = Self] {
    @builtin(.ModuloAssignMethod)
    mutating func modAssign(other: Rhs)
}

@builtin(.BitwiseAndAssignProtocol)
public protocol BitwiseAndAssign[Rhs = Self] {
    @builtin(.BitwiseAndAssignMethod)
    mutating func bitwiseAndAssign(other: Rhs)
}

@builtin(.BitwiseOrAssignProtocol)
public protocol BitwiseOrAssign[Rhs = Self] {
    @builtin(.BitwiseOrAssignMethod)
    mutating func bitwiseOrAssign(other: Rhs)
}

@builtin(.BitwiseXorAssignProtocol)
public protocol BitwiseXorAssign[Rhs = Self] {
    @builtin(.BitwiseXorAssignMethod)
    mutating func bitwiseXorAssign(other: Rhs)
}

@builtin(.ShiftLeftAssignProtocol)
public protocol LeftShiftAssign[Rhs] {
    @builtin(.ShiftLeftAssignMethod)
    mutating func shiftLeftAssign(by count: Rhs)
}

@builtin(.ShiftRightAssignProtocol)
public protocol RightShiftAssign[Rhs] {
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
