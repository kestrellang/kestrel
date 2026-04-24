// test: diagnostics
// stdlib: false

module Test

protocol Clone {
    func clone() -> Self
}
func duplicateIt[T](x: T) -> T where T: Clone {
    return x.clone()
}
