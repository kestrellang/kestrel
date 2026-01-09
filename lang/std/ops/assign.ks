// Compound assignment operator protocols

module std.ops

// TODO: Add back 
//@operator(+=)
public protocol AddAssign[Rhs = Self] {
    func addAssign(other: Rhs)
}

// TODO: Add back 
//@operator(-=)
public protocol SubtractAssign[Rhs = Self] {
    func subtractAssign(other: Rhs)
}

// TODO: Add back 
//@operator(*=)
public protocol MultiplyAssign[Rhs = Self] {
    func multiplyAssign(other: Rhs)
}

// TODO: Add back 
//@operator(/=)
public protocol DivideAssign[Rhs = Self] {
    func divideAssign(other: Rhs)
}

// TODO: Add back 
//@operator(%=)
public protocol ModuloAssign[Rhs = Self] {
    func modAssign(other: Rhs)
}

// TODO: Add back 
//@operator(&=)
public protocol BitwiseAndAssign[Rhs = Self] {
    func bitwiseAndAssign(other: Rhs)
}

// TODO: Add back 
//@operator(|=)
public protocol BitwiseOrAssign[Rhs = Self] {
    func bitwiseOrAssign(other: Rhs)
}

// TODO: Add back 
//@operator(^=)
public protocol BitwiseXorAssign[Rhs = Self] {
    func bitwiseXorAssign(other: Rhs)
}

// TODO: Add back 
//@operator(<<=)
public protocol LeftShiftAssign[Rhs = Int] {
    func shiftLeftAssign(by count: Rhs)
}

// TODO: Add back 
//@operator(>>=)
public protocol RightShiftAssign[Rhs = Int] {
    func shiftRightAssign(by count: Rhs)
}

// Default implementations from base operators
extension Addable[Rhs]: AddAssign[Rhs] where Output = Self {
    func addAssign(other: Rhs) {
        self = self.add(other)
    }
}

extension Subtractable[Rhs]: SubtractAssign[Rhs] where Output = Self {
    func subtractAssign(other: Rhs) {
        self = self.subtract(other)
    }
}

extension Multipliable[Rhs]: MultiplyAssign[Rhs] where Output = Self {
    func multiplyAssign(other: Rhs) {
        self = self.multiply(other)
    }
}

extension Divisible[Rhs]: DivideAssign[Rhs] where Output = Self {
    func divideAssign(other: Rhs) {
        self = self.divide(other)
    }
}

extension Modulo[Rhs]: ModuloAssign[Rhs] where Output = Self {
    func modAssign(other: Rhs) {
        self = self.mod(other)
    }
}

extension BitwiseAnd[Rhs]: BitwiseAndAssign[Rhs] where Output = Self {
    func bitwiseAndAssign(other: Rhs) {
        self = self.bitwiseAnd(other)
    }
}

extension BitwiseOr[Rhs]: BitwiseOrAssign[Rhs] where Output = Self {
    func bitwiseOrAssign(other: Rhs) {
        self = self.bitwiseOr(other)
    }
}

extension BitwiseXor[Rhs]: BitwiseXorAssign[Rhs] where Output = Self {
    func bitwiseXorAssign(other: Rhs) {
        self = self.bitwiseXor(other)
    }
}

extension LeftShift[Rhs]: LeftShiftAssign[Rhs] where Output = Self {
    func shiftLeftAssign(by count: Rhs) {
        self = self.shiftLeft(by: count)
    }
}

extension RightShift[Rhs]: RightShiftAssign[Rhs] where Output = Self {
    func shiftRightAssign(by count: Rhs) {
        self = self.shiftRight(by: count)
    }
}
