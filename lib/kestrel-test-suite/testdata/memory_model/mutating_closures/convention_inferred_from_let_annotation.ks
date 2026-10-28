// test: execution
// stdlib: true

module Test

import std.numeric.(Int64)

struct Counter { var n: Int64 }

func bumpCounter(mutating c: Counter, with f: (mutating Counter) -> ()) {
    f(c);
}

// Regression (#178): the closure literal omits `mutating`; its param
// convention is inferred from the `let` annotation
// `(mutating Counter) -> ()` — the SAME inference that already worked at
// argument position. Previously this falsely fired E201 ("cannot assign to
// immutable field 'n'") because the convention upgrade only ran at call sites,
// not on an annotated binding's coercion.
@main
func main() -> lang.i64 {
    var c = Counter(n: 10);
    let f: (mutating Counter) -> () = { (x) in x.n = x.n + 10; };
    bumpCounter(c, with: f);
    bumpCounter(c, with: f);
    if c.n != 30 { return 1 }
    0
}
