// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 1 0 0 true\n

// A conformance on a default-filled target (`extend Array[T]: P` is
// `Array[T, GlobalAllocator]: P`) accepts witnesses that spell `Self` with the
// args left off, through a plain name, an alias default, or type sugar.
module Test

protocol Make { static func make() -> Self }

struct Pair[T, U = Int64] { var n: Int64 }

extend Pair[T]: Make { static func make() -> Pair[T] { Pair[T](n: 1) } }
extend Array[T]: Make { static func make() -> Array[T] { Array[T]() } }

protocol Make2 { static func make2() -> Self }
extend [T]: Make2 { static func make2() -> [T] { [] } }
extend Optional[T]: Make2 { static func make2() -> T? { .None } }

@main
func main() {
    let p = Pair[Bool].make();
    let a = Array[Int64].make();
    let b = Array[Bool].make2();
    let o = Optional[Int64].make2();
    print("\(p.n) \(a.count) \(b.count) \(o.isNone)\n");
}
