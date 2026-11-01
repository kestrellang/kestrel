// test: execution
// stdlib: true
// expect-exit: 0
//
// A literal whose type is a generic parameter `T: ExpressibleByIntLiteral` must
// call the substituted type's `init(intLiteral:)`. Today literal lowering only
// resolves the init for a concrete named type, so a `T`-typed literal is emitted
// as a raw i64 and bit-reinterpreted as Meters (v = 7, exit 75) — the init's
// `* 100` never runs.

module Test

struct Meters: ExpressibleByIntLiteral {
    var v: Int64
    init(intLiteral value: lang.i64) { self.v = Int64(intLiteral: value) * 100 }
}

func make[T]() -> T where T: ExpressibleByIntLiteral { 7 }

@main
func main() -> Int64 {
    let m: Meters = make();
    m.v - 700
}
