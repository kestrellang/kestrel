// test: diagnostics

// Default arguments are inlined at each call site. A default that calls its
// own function without passing that argument would inline forever (it used
// to overflow the compiler's stack).

module Test

func f(x: Int64 = f()) -> Int64 { x } // ERROR: default argument value depends on itself

func g(x: Int64 = g(1)) -> Int64 { x }

@main
func main() -> Int64 {
    f() + g()
}
