// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b: `match` over Optional[&T] — the .Some arm binds the
// payload REF (no decay), the .None arm runs without one.
module Test

func describe(o: Optional[&Int64]) -> Int64 {
    match o {
        .Some(r) => r + 100,
        .None => -1
    }
}

@main
func main() {
    var x = 7;
    let r = &x;
    let sval: Optional[&Int64] = .Some(r);
    let nval: Optional[&Int64] = .None;
    if describe(sval) != 107 {
        fatalError("Some arm broke");
    }
    if describe(nval) != -1 {
        fatalError("None arm broke");
    }
    print("ok");
}

// CHECK: ok
