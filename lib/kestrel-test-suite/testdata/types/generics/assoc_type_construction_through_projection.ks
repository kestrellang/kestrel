// test: execution
// stdlib: true
// expect-exit: 0

// `B.Item(args)` constructs the associated type through its init
// requirement. It used to lower as a static method `Item` on `B` — E102
// "no member 'Item' on type 'B'".

module Test
import std.numeric.Int64
protocol Made { var n: Int64 { get } init(n v: Int64) }
protocol Factory { type Item: Made }
struct Thing: Made { var n: Int64  init(n v: Int64) { self.n = v; } }
struct F: Factory { type Item = Thing }
func build[B](b: B) -> Int64 where B: Factory { B.Item(n: 6).n }
@main
func main() -> lang.i64 { if build(F()) == 6 { 0 } else { 1 } }
