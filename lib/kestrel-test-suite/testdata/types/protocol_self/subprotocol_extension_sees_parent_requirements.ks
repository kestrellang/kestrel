// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Inside `extend B`, `self` is B's abstract `Self`, which conforms to B's
// parent A, so A's requirements are members of `self` (as they are for a
// `T: B` param).

module Test

protocol A { func a() -> Int64 }
protocol B: A { func b() -> Int64 }

extend B {
    func ab() -> Int64 { self.a() + self.b() }
}

struct S: A, B {
    func a() -> Int64 { 1 }
    func b() -> Int64 { 2 }
}

@main
func main() -> lang.i64 {
    if S().ab() != 3 { return 1 }
    0
}
