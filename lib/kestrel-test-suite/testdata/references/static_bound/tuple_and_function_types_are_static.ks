// test: execution
// backends: cranelift,llvm
// stdlib: true

// 2a cuts pinned: tuples fold their elements (all-Static tuple passes a
// Static bound) and function types are Static (the capture-derived
// Static bit is 2c).

module Test

func requireStatic[T](consuming x: T) -> T where T: Static {
    x
}

@main
func main() {
    let pair = requireStatic((1, "two"));
    if pair.0 != 1 {
        fatalError("tuple through Static bound broke");
    }
    let f = requireStatic({ (n: Int64) in n * 2 });
    if f(21) != 42 {
        fatalError("closure through Static bound broke");
    }
    print("ok");
}

// CHECK: ok
