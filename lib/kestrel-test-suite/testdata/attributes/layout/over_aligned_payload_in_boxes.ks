// test: execution
// stdlib: true
// backends: cranelift,llvm
//
// A heap box places its payload after a one-word header, rounded up to the
// payload's alignment. An `@align(32)` capture must be found where the
// layout put it, in a `consuming` closure's unique box and in an
// `escaping` closure's shared box alike.

module Main

import std.numeric.(Int64)

@align(32)
struct Wide { var a: Int64; var b: Int64 }

func runOnce(consuming f: consuming () -> Int64) -> Int64 { f() }

func makeReader(w: Wide) -> escaping () -> Int64 {
    { w.a + w.b }
}

@main
func main() -> lang.i64 {
    let w = Wide(a: 40, b: 2);
    let tag = 7;
    let once: consuming () -> Int64 = { tag + w.a + w.b };
    if runOnce(once) != 49 { return 1 }

    let read = makeReader(Wide(a: 30, b: 3));
    if read() != 33 { return 2 }
    if read() != 33 { return 3 }
    0
}
