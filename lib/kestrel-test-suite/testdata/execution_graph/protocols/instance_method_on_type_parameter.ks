// test: diagnostics
// stdlib: false

module Test

protocol Add {
    func add(other: Self) -> Self
}

func addThem[T](a: T, b: T) -> T where T: Add {
    return a.add(b)
}
