// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b flagship: Optional[&T] wrap/unwrap roundtrip. The payload
// ref aliases the source var (may-alias), the pattern binding reads
// through it, and .None forms without a ref.
module Test

@main
func main() {
    var x = 41;
    let r = &x;
    let o: Optional[&Int64] = .Some(r);
    if let .Some(v) = o {
        x = 42;                 // may-alias write, visible through v
        if v != 42 {
            fatalError("payload ref did not alias the source: \(v)");
        }
    } else {
        fatalError("Some matched as None");
    }
    let n: Optional[&Int64] = .None;
    if let .Some(_) = n {
        fatalError("None matched as Some");
    }
    print("ok");
}

// CHECK: ok
