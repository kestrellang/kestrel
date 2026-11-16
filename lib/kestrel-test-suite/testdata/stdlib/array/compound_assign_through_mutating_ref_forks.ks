// test: execution
// stdlib: true

// `s.xs(1) += 1` through a `&mutating` binding must fork the array's shared
// storage, like `b.xs(1) += 1` and `s.xs(1) = s.xs(1) + 1` already do. Today
// the write lands in storage `a` still shares, so `a` sees it.
module Test

struct Holder { var xs: [Int64] }

@main
func main() -> Int64 {
    var a = Holder(xs: [0, 0]);
    var b = a;
    let s = &mutating b;
    s.xs(1) += 1;
    if b.xs(1) != 1 { return 1; }
    if a.xs(1) != 0 { return 2; }
    0
}
