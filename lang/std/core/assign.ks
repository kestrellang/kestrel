// Compound assignment operator protocols

module std.core

public protocol AddAssign[Rhs = Self] {
    mutating func addAssign(other: Rhs)
}

public protocol SubtractAssign[Rhs = Self] {
    mutating func subtractAssign(other: Rhs)
}

public protocol MultiplyAssign[Rhs = Self] {
    mutating func multiplyAssign(other: Rhs)
}

public protocol DivideAssign[Rhs = Self] {
    mutating func divideAssign(other: Rhs)
}

public protocol ModuloAssign[Rhs = Self] {
    mutating func modAssign(other: Rhs)
}

public protocol BitwiseAndAssign[Rhs = Self] {
    mutating func bitwiseAndAssign(other: Rhs)
}

public protocol BitwiseOrAssign[Rhs = Self] {
    mutating func bitwiseOrAssign(other: Rhs)
}

public protocol BitwiseXorAssign[Rhs = Self] {
    mutating func bitwiseXorAssign(other: Rhs)
}

public protocol LeftShiftAssign[Rhs] {
    mutating func shiftLeftAssign(by count: Rhs)
}

public protocol RightShiftAssign[Rhs] {
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
