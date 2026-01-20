// Compound assignment operator protocols

module std.core

// TODO: Add builtin support
//@builtin(.AddAssignOperatorProtocol)
public protocol AddAssign[Rhs = Self] {
    //@builtin(.AddAssignOperatorMethod)
    mutating func addAssign(other: Rhs)
}

// TODO: Add builtin support
//@builtin(.SubtractAssignOperatorProtocol)
public protocol SubtractAssign[Rhs = Self] {
    //@builtin(.SubtractAssignOperatorMethod)
    mutating func subtractAssign(other: Rhs)
}

// TODO: Add builtin support
//@builtin(.MultiplyAssignOperatorProtocol)
public protocol MultiplyAssign[Rhs = Self] {
    //@builtin(.MultiplyAssignOperatorMethod)
    mutating func multiplyAssign(other: Rhs)
}

// TODO: Add builtin support
//@builtin(.DivideAssignOperatorProtocol)
public protocol DivideAssign[Rhs = Self] {
    //@builtin(.DivideAssignOperatorMethod)
    mutating func divideAssign(other: Rhs)
}

// TODO: Add builtin support
//@builtin(.ModuloAssignOperatorProtocol)
public protocol ModuloAssign[Rhs = Self] {
    //@builtin(.ModuloAssignOperatorMethod)
    mutating func modAssign(other: Rhs)
}

// TODO: Add builtin support
//@builtin(.BitwiseAndAssignOperatorProtocol)
public protocol BitwiseAndAssign[Rhs = Self] {
    //@builtin(.BitwiseAndAssignOperatorMethod)
    mutating func bitwiseAndAssign(other: Rhs)
}

// TODO: Add builtin support
//@builtin(.BitwiseOrAssignOperatorProtocol)
public protocol BitwiseOrAssign[Rhs = Self] {
    //@builtin(.BitwiseOrAssignOperatorMethod)
    mutating func bitwiseOrAssign(other: Rhs)
}

// TODO: Add builtin support
//@builtin(.BitwiseXorAssignOperatorProtocol)
public protocol BitwiseXorAssign[Rhs = Self] {
    //@builtin(.BitwiseXorAssignOperatorMethod)
    mutating func bitwiseXorAssign(other: Rhs)
}

// TODO: Add builtin support
//@builtin(.LeftShiftAssignOperatorProtocol)
public protocol LeftShiftAssign[Rhs] {
    //@builtin(.LeftShiftAssignOperatorMethod)
    mutating func shiftLeftAssign(by count: Rhs)
}

// TODO: Add builtin support
//@builtin(.RightShiftAssignOperatorProtocol)
public protocol RightShiftAssign[Rhs] {
    //@builtin(.RightShiftAssignOperatorMethod)
    mutating func shiftRightAssign(by count: Rhs)
}

// TODO: Protocol extensions with `where Output = Self` don't work yet.
// The compiler sees Self as the protocol type rather than the implementing type.
// Default implementations from base operators:
//
// extend Addable[Rhs]: AddAssign[Rhs] where Output = Self {
//     public mutating func addAssign(other: Rhs) {
//         self = self.add(other)
//     }
// }
//
// extend Subtractable[Rhs]: SubtractAssign[Rhs] where Output = Self {
//     public mutating func subtractAssign(other: Rhs) {
//         self = self.subtract(other)
//     }
// }
//
// extend Multipliable[Rhs]: MultiplyAssign[Rhs] where Output = Self {
//     public mutating func multiplyAssign(other: Rhs) {
//         self = self.multiply(other)
//     }
// }
//
// extend Divisible[Rhs]: DivideAssign[Rhs] where Output = Self {
//     public mutating func divideAssign(other: Rhs) {
//         self = self.divide(other)
//     }
// }
//
// extend Modulo[Rhs]: ModuloAssign[Rhs] where Output = Self {
//     public mutating func modAssign(other: Rhs) {
//         self = self.modulo(other)
//     }
// }
//
// extend BitwiseAnd[Rhs]: BitwiseAndAssign[Rhs] where Output = Self {
//     public mutating func bitwiseAndAssign(other: Rhs) {
//         self = self.bitwiseAnd(other)
//     }
// }
//
// extend BitwiseOr[Rhs]: BitwiseOrAssign[Rhs] where Output = Self {
//     public mutating func bitwiseOrAssign(other: Rhs) {
//         self = self.bitwiseOr(other)
//     }
// }
//
// extend BitwiseXor[Rhs]: BitwiseXorAssign[Rhs] where Output = Self {
//     public mutating func bitwiseXorAssign(other: Rhs) {
//         self = self.bitwiseXor(other)
//     }
// }
//
// extend LeftShift[Rhs]: LeftShiftAssign[Rhs] where Output = Self {
//     public mutating func shiftLeftAssign(by count: Rhs) {
//         self = self.shiftLeft(by: count)
//     }
// }
//
// extend RightShift[Rhs]: RightShiftAssign[Rhs] where Output = Self {
//     public mutating func shiftRightAssign(by count: Rhs) {
//         self = self.shiftRight(by: count)
//     }
// }
