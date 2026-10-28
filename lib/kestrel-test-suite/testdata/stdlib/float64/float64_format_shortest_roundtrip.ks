// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Default (no-precision) printing emits the SHORTEST decimal that round-trips
// back to the exact stored value (Dragon4 boundary search), like Rust/Swift/
// Python. `0.1 + 0.2` must show its true value, not a precision-6 "0.3".

module Test

import std.numeric.Float64

func f(v: Float64) -> Float64 { v }

@main
func main() -> lang.i32 {
    if "\(f(0.1))" != "0.1" { return 1 };
    if "\(f(0.1) + f(0.2))" != "0.30000000000000004" { return 2 };
    if "\(f(1.0) / f(3.0))" != "0.3333333333333333" { return 3 };
    if "\(f(1.5))" != "1.5" { return 4 };
    if "\(f(100.0))" != "100" { return 5 };
    if "\(f(2.0))" != "2" { return 6 };
    if "\(f(0.5))" != "0.5" { return 7 };
    if "\(f(3.14159))" != "3.14159" { return 8 };
    if "\(f(-2.5))" != "-2.5" { return 9 };
    if "\(f(1.0e20))" != "1e20" { return 10 };       // shortest scientific
    if "\(f(123.456))" != "123.456" { return 11 };
    0
}
