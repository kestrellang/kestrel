// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 5 6\n

// A static call on a generic type named bare (`W.make()`) takes the
// declared default for every param nothing pins, as `W()` does, so a bound
// on that param holds. It used to stay open and fail `A: P` (E107); with
// `String[A = GlobalAllocator]` that broke `String.fromBytes...` calls.
module Test

protocol P {}
struct D: P {}

struct W[A = D] where A: P {
    var a: Int64
    static func make() -> Int64 { 5 }
}

extend W[D] {
    static func makeDefault() -> Int64 { 6 }
}

@main
func main() {
    print("\(W.make()) \(W.makeDefault())\n");
}
