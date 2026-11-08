// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A static method overloaded only by parameter type, through instantiations
// of one parameterized protocol, should resolve by argument type the way
// `init(from:)` does. Today the call fails with a misleading E116 ("static
// member cannot be used on an instance"), which blocks `Int8.exactly(_:)`.
protocol Exactly[From] {
    static func exactly(value: From) -> Self
}

struct Meters: Exactly[Int64], Exactly[Bool] {
    var value: Int64
    static func exactly(value: Int64) -> Meters { Meters(value: value) }
    static func exactly(value: Bool) -> Meters { Meters(value: 1) }
}

@main
func main() -> lang.i64 {
    let n: Int64 = 5;
    if Meters.exactly(n).value != 5 { return 1 }
    if Meters.exactly(true).value != 1 { return 2 }
    0
}
