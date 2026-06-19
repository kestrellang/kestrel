// test: execution
// stdlib: true
// backends: cranelift,llvm

// The composition payoff: generic ITERATOR algorithms over ref Items work
// once `&T` forwards Equatable/Comparable (stdlib `extend &T:` in
// core/ref.ks). `xs.refs()` yields `Item = &Int64`; `contains` requires
// `Item: Equatable`, `min` requires `Item: Comparable` — both dispatch the
// pointee's witnesses through the forwarding extensions.
module Test

@main
func main() -> Int64 {
    let xs = [10, 20, 30];

    var x = 20;
    let needle = &x;
    var it = xs.refs();
    if not it.contains(needle) { return 1; }

    var y = 99;
    let missing = &y;
    var it2 = xs.refs();
    if it2.contains(missing) { return 2; }

    var it3 = xs.refs();
    let m = it3.min();
    if let .Some(r) = m {
        if not (r == 10) { return 3; }
    } else {
        return 4;
    }
    0
}
