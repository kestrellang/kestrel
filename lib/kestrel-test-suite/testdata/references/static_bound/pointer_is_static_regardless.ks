// test: execution
// backends: cranelift,llvm
// stdlib: true

// Pointer[T] stays Static regardless of T (it stores only a raw address,
// never a T) — the load-bearing position-gated arg recursion. With T
// itself non-Static, Pointer[T] still passes a Static bound.

module Test

struct Handle: not Static {
    var id: Int64
}

func requireStatic[T](x: T) -> T where T: Static {
    x
}

@main
func main() {
    var h = Handle(id: 5);
    let p = requireStatic(Pointer(to: h));
    if p.value.id != 5 {
        fatalError("pointer through Static bound should still read");
    }
    var n: Int64 = 11;
    let q = requireStatic(Pointer(to: n));
    if q.value != 11 {
        fatalError("plain pointee through Static bound should still read");
    }
    print("ok");
}

// CHECK: ok
