// test: diagnostics
// stdlib: false

module Test

protocol Pair {
    func pair() -> (Self, Self)
}
func getPair[T](x: T) -> (T, T) where T: Pair {
    return x.pair()
}
func main() {}
