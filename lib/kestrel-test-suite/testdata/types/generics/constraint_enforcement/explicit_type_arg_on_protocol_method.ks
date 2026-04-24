// test: diagnostics
// stdlib: false

module Test

protocol Mapper {
    func map[U](x: lang.i64) -> U
}
func apply[T](x: T) -> lang.i64 where T: Mapper {
    return x.map[lang.i64](1)
}
func main() {}
