// test: execution
// stdlib: true
// backends: cranelift,llvm

// An unlabeled trailing closure fills the one closure-typed parameter it
// can, even a labeled one: `xs.iter().filter { … }` for `filter(where:)`.
// Free functions, methods, generic methods and inits all bind the same way.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct P {
    var n: Int64
}

struct Button {
    var tapped: Int64

    init(title t: Int64, onTap f: () -> Int64) {
        self.tapped = t + f();
    }
}

func run(times n: Int64, body f: (Int64) -> Int64) -> Int64 { f(n) }

func both(first f: () -> Int64, then s: () -> Int64) -> Int64 { f() * 10 + s() }

func label(id: Int64, onA a: () -> Int64 = { 1 }) -> Int64 { id + a() }

@main
func main() -> lang.i64 {
    let xs = [P(n: 1), P(n: 2), P(n: 3)];

    // Stdlib adapters with labeled closure parameters.
    let big = xs.iter().filter { it.n > 1 }.count();
    if big != 2 { return 1; }
    let tens = xs.refs().map { it.n * 10 }.collect();
    if tens(2) != 30 { return 2; }
    let sum = xs.refs().fold(from: 0) { (acc, r) in acc + r.n };
    if sum != 6 { return 3; }

    // Free functions: after other args, after a labeled closure, and in
    // place of a defaulted closure parameter.
    let r = run(times: 4) { it * 2 };
    if r != 8 { return 4; }
    let b2 = both(first: { 1 }) { 2 };
    if b2 != 12 { return 5; }
    let l = label(5) { 10 };
    if l != 15 { return 6; }
    if label(5) != 6 { return 7; }

    // An init.
    let b = Button(title: 3) { 4 };
    if b.tapped != 7 { return 8; }
    0
}
