// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b: the slot-taint G1 closure must NOT reject the flagship —
// a PARAM-rooted ref-bearing value stored in a var (cursor reassigned in
// place) stays returnable and aliases the caller's storage.
module Test

func wrapThroughVar(a: Int64) -> Optional[&Int64] {
    let r = &a;
    var o: Optional[&Int64] = .Some(r);
    o
}

@main
func main() {
    var x = 5;
    let o = wrapThroughVar(x);
    x = 8;
    if let .Some(v) = o {
        if v != 8 {
            fatalError("param-rooted var roundtrip lost aliasing: \(v)");
        }
    } else {
        fatalError("Some matched as None");
    }
    print("ok");
}

// CHECK: ok
