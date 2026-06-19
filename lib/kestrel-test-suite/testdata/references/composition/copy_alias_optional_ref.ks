// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b ruling: refs are Copyable as payload values — a copied
// Optional[&T] bit-copies the pointer, so both values alias the SAME
// storage (may-alias model).
module Test

func readThrough(o: Optional[&Int64]) -> Int64 {
    match o {
        .Some(r) => r,
        .None => -1
    }
}

@main
func main() {
    var x = 1;
    let r = &x;
    let o: Optional[&Int64] = .Some(r);
    let o2 = o;     // Copyable: bit-copy, aliases the same x
    x = 9;
    let a = readThrough(o);
    let b = readThrough(o2);
    if a != 9 or b != 9 {
        fatalError("copied Optional[&T] did not alias: \(a) \(b)");
    }
    print("ok");
}

// CHECK: ok
