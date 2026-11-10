// test: execution
// stdlib: true
// expect-stdout: 41 99\n

// Inside `extend P`, `self.b()` names the requirement `b`, not tie with
// `b`'s own default: it dispatches to the conformer's witness, so a type
// that overrides `b` is seen by the other defaults.

module Test

protocol P {
    func a() -> Int64
    func b() -> Int64
}

extend P {
    func b() -> Int64 { self.a() * 10 }
    func c(x: Int64) -> Int64 { self.b() + x }
}

struct S: P { func a() -> Int64 { 4 } }
struct Over: P {
    func a() -> Int64 { 1 }
    func b() -> Int64 { 98 }
}

@main
func main() {
    println("\(S().c(1)) \(Over().c(1))");
}
