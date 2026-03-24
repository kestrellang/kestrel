// test: diagnostics
// stdlib: false

module Test

protocol Add {
    func add(other: Self) -> Self
}

protocol Mul {
    func mul(other: Self) -> Self
}

func compute[T](a: T, b: T) -> T where T: Add and Mul {
    let sum = a.add(b);
    return sum.mul(b)
}
