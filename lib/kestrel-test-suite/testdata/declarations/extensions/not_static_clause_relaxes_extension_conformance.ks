// test: execution
// stdlib: true
// expect-exit: 0

// `where W: not Static` on an extension is a relaxation, as on a function:
// W *may* hold references, so the conformance covers static and non-static
// W alike. It used to be read as "W must be non-Static", which dropped the
// conformance for every static type (exit 10).

module Test

import std.numeric.Int64

protocol Reads[W] {
    func read(w: W) -> Int64
}

struct Holder {
    var x: Int64
}

extend[W] Int64: Reads[W] where W: not Static {
    func read(w: W) -> Int64 { self + 1 }
}

func readFive[W](w: W) -> Int64 where W: not Static {
    let five: Int64 = 5;
    five.read(w)
}

@main
func main() -> lang.i32 {
    let h = Holder(x: 1);
    let one: Int64 = 1;
    if one.read(h) != 2 { return 10 }
    let r = &h;
    if one.read(r) != 2 { return 11 }
    if readFive(r) != 6 { return 12 }
    if readFive(h) != 6 { return 13 }
    0
}
