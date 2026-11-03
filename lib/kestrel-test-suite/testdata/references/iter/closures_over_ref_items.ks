// test: execution
// stdlib: true
// backends: cranelift,llvm

// A closure can take a reference: iterator adapters over `refs()` /
// `mutableRefs()` pass each `&T` / `&mutating T` item straight to the
// closure, with no copy. Only a function type that RETURNS a ref is
// rejected (E491, see escape/ref_function_value_rejected.ks).
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct P {
    var n: Int64
}

@main
func main() -> lang.i64 {
    var xs = [P(n: 1), P(n: 2), P(n: 3)];

    // fold / map / filter / forEach over shared refs.
    let sum = xs.refs().fold(from: 0, by: { (acc, r) in acc + r.n });
    if sum != 6 { return 1; }
    var total = 0;
    let tens = xs.refs().map(as: { (r) in r.n * 10 });
    for v in tens { total += v; }
    if total != 60 { return 2; }
    let big = xs.refs().filter(where: { (r) in r.n > 1 }).count();
    if big != 2 { return 3; }
    var seen = 0;
    xs.refs().forEach({ (r) in seen += r.n; });
    if seen != 6 { return 4; }

    // A closure over mutable refs writes the elements in place.
    xs.mutableRefs().forEach({ (r) in r.n += 1; });
    if xs(0).n != 2 { return 5; }
    if xs(2).n != 4 { return 6; }
    0
}
