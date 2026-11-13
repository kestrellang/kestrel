// test: execution
// stdlib: true
// expect-stdout: 3\n2\n7.5\n

// A subscript result peeled through `Indirection` (`xs(i).toOwned()`, an
// `Array[Int64]` element passed as `&Slice[Int64]`) is a single-use ref: it
// ends with the call that used it, so a branch right after (a `guard`, a
// failable init, `??`) is not a false E497.

module Test

import std.collections.(Array, Slice)

struct Q {
    let n: Int64
    init(v: &Slice[Int64])? {
        if v.isEmpty { return null }
        self.n = v.count
    }
}

func size(s: String) -> Int64? { .Some(s.bytes.count) }

@main
func main() {
    let xs: Array[String] = ["ab", "cde"];
    let i: Int64 = 1;
    guard let some n = size(xs(i).toOwned()) else { return; }
    println(n);
    let xss: Array[Array[Int64]] = [[1], [2, 3]];
    let q = Q(xss(1))?.n ?? 0;
    println(q);
    let pieces = "1 2 7.5".split(by: " ").collect();
    println(Float64(parsing: pieces(2)) ?? 0.0);
}
