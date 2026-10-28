// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b: without a type-side pin, `.Some(r)` DECAYS — the payload
// is an owned snapshot of the pointee (Optional[Int64]), not an alias.
// (With an Optional[&Int64] annotation anywhere downstream, inference
// pins the ref type instead — see optional_ref_roundtrip.)
module Test

@main
func main() {
    var x = 1;
    let r = &x;
    let o = Optional.Some(r);
    x = 99;
    if let .Some(v) = o {
        if v != 1 {
            fatalError("unpinned .Some(r) should snapshot, got \(v)");
        }
    } else {
        fatalError("Some matched as None");
    }
    print("ok");
}

// CHECK: ok
