// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b (bare-ref-as-T): a generic `-> T` callee instantiated at
// `T = &U` returns the ref by value; the caller registers it like a
// ret_borrow result. A `let` of the result DECAYS (snapshot, stage-1.5
// binding rule); reading the call result in place aliases.
module Test

func identity[T](x: T) -> T where T: not Static {
    x
}

@main
func main() {
    var x = 41;
    let r = &x;
    let o: Optional[&Int64] = .Some(r);
    let v = o.unwrap();
    if v != 41 {
        fatalError("unwrap snapshot broke: \(v)");
    }
    x = 42;
    if v != 41 {
        fatalError("snapshot must not alias: \(v)");
    }
    if o.unwrap() != 42 {
        fatalError("in-place unwrap read must alias");
    }
    var y = 5;
    let r2 = &y;
    let w = identity[&Int64](r2);
    if w != 5 {
        fatalError("identity[&T] snapshot broke: \(w)");
    }
    print("ok");
}

// CHECK: ok
