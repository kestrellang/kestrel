// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b root rule: a ref-bearing aggregate rooted at a BORROWED
// param is returnable — the caller's storage outlives the call.
module Test

func wrapRef(a: Int64) -> Optional[&Int64] {
    let r = &a;
    let o: Optional[&Int64] = .Some(r);
    o
}

@main
func main() {
    var x = 5;
    let o = wrapRef(x);
    x = 8;
    if let .Some(r) = o {
        if r != 8 {
            fatalError("param-rooted payload did not alias caller storage: \(r)");
        }
    } else {
        fatalError("Some matched as None");
    }
    print("ok");
}

// CHECK: ok
