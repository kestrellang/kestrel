// test: execution
// backends: cranelift,llvm
// stdlib: true

// Tuples fold their elements (an all-Static tuple passes a Static bound),
// and owning closure types (`escaping`) are Static. View closures are not
// (references 2c, docs/design/closure-fields.md).

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
    let double: escaping (Int64) -> Int64 = { (n: Int64) in n * 2 };
    let f = requireStatic(double);
    if f(21) != 42 {
        fatalError("closure through Static bound broke");
    }
    print("ok");
}

// CHECK: ok
