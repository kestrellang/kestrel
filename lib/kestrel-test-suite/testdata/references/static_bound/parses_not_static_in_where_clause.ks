// test: execution
// backends: cranelift,llvm
// stdlib: true

// `where T: not Static` relaxes the (implicit) Static bound on a type
// param — need-not, not must-not — so Static arguments still flow in
// (the `T: not Copyable` convention; cf. Pointer[T]).

module Test

func passThrough[T](consuming x: T) -> T where T: not Static {
    x
}

@main
func main() {
    let n = passThrough(42);
    if n != 42 {
        fatalError("relaxed generic should accept a Static argument");
    }
    print("ok");
}

// CHECK: ok
