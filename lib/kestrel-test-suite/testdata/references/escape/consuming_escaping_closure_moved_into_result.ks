// test: execution
// stdlib: true
// expect-exit: 0

// Moving a `consuming` escaping closure into a returned value that also
// carries a reference is fine: the closure is owned by the result, not
// borrowed from the parameter. This currently reports a false E496
// ("reference rooted at consuming parameter `f`"), because a function
// value is treated as possibly holding a reference to itself. std works
// around it in Array.split(where:) by building the view directly.

module Test

struct Holder: not Static {
    let r: &Int64
    let f: escaping (Int64) -> Bool
}

func make(x: Int64, consuming f: escaping (Int64) -> Bool) -> Holder {
    let r = &x;
    Holder(r: r, f: f)
}

@main
func main() -> Int64 {
    let n: Int64 = 3;
    let h = make(n, { (v) in v > 1 });
    if h.f(h.r) == false { return 1 }
    0
}
