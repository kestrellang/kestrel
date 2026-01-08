// Compound assignment operator protocols

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
extension Addable[Rhs] where Output = Self: AddAssign[Rhs] {
    func addAssign(other: Rhs) {
        self = self.add(other)
    }
}

extension Subtractable[Rhs] where Output = Self: SubtractAssign[Rhs] {
    func subtractAssign(other: Rhs) {
        self = self.subtract(other)
    }
}

extension Multipliable[Rhs] where Output = Self: MultiplyAssign[Rhs] {
    func multiplyAssign(other: Rhs) {
        self = self.multiply(other)
    }
}

extension Divisible[Rhs] where Output = Self: DivideAssign[Rhs] {
    func divideAssign(other: Rhs) {
        self = self.divide(other)
    }
}

extension Modulo[Rhs] where Output = Self: ModuloAssign[Rhs] {
    func modAssign(other: Rhs) {
        self = self.mod(other)
    }
}

extension BitwiseAnd[Rhs] where Output = Self: BitwiseAndAssign[Rhs] {
    func bitwiseAndAssign(other: Rhs) {
        self = self.bitwiseAnd(other)
    }
}

extension BitwiseOr[Rhs] where Output = Self: BitwiseOrAssign[Rhs] {
    func bitwiseOrAssign(other: Rhs) {
        self = self.bitwiseOr(other)
    }
}

extension BitwiseXor[Rhs] where Output = Self: BitwiseXorAssign[Rhs] {
    func bitwiseXorAssign(other: Rhs) {
        self = self.bitwiseXor(other)
    }
}

extension LeftShift[Rhs] where Output = Self: LeftShiftAssign[Rhs] {
    func shiftLeftAssign(by count: Rhs) {
        self = self.shiftLeft(by: count)
    }
}

extension RightShift[Rhs] where Output = Self: RightShiftAssign[Rhs] {
    func shiftRightAssign(by count: Rhs) {
        self = self.shiftRight(by: count)
    }
}
