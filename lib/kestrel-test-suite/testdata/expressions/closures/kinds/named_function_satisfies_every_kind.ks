// test: execution
// stdlib: true
// expect-exit: 0

// "A capture-free closure (or a named function) is a bare function pointer: it
// has no environment, satisfies every kind, and escapes freely."
// (docs/design/closures.md — "The Four Kinds"). Pins that a named func value
// is accepted in every kind slot — parameter, `let` annotation, and return
// position — without allocating a shared environment.
module Test

import std.numeric.(Int64)

func seven() -> Int64 { 7 }

func callNormal(f: () -> Int64) -> Int64 { f() }

func callMutating(mutating f: mutating () -> Int64) -> Int64 { f() }

func callConsuming(consuming f: consuming () -> Int64) -> Int64 { f() }

func callEscaping(f: escaping () -> Int64) -> Int64 { f() }

// A capture-free function pointer escapes freely, so it may be returned.
func escapingSeven() -> escaping () -> Int64 { seven }

@main
func main() -> lang.i64 {
    if callNormal(seven) != 7 { return 1 }
    if callMutating(seven) != 7 { return 2 }
    if callConsuming(seven) != 7 { return 3 }
    if callEscaping(seven) != 7 { return 4 }

    let e: escaping () -> Int64 = seven;
    if e() != 7 { return 5 }

    var m: mutating () -> Int64 = seven;
    if m() != 7 { return 6 }

    let c: consuming () -> Int64 = seven;
    if c() != 7 { return 7 }

    if (escapingSeven())() != 7 { return 8 }

    0
}
