// test: execution
// stdlib: true
// expect-exit: 0

// An undeclared one-letter name in an extension's protocol args names the
// type in scope. It used to silently declare a fresh type param that
// shadowed struct `V`, so `view.n` failed with "no member 'n' on type 'V'".

module Test

import std.numeric.Int64

protocol Reads[W] {
    func read(view: W) -> Int64
}

struct V {
    var n: Int64
}

extend Int64: Reads[V] {
    func read(view: V) -> Int64 { view.n + self }
}

@main
func main() -> lang.i32 {
    let two: Int64 = 2;
    if two.read(V(n: 40)) != 42 { return 10 }
    0
}
