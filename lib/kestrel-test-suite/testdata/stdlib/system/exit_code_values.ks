// test: execution
// stdlib: true

// ExitCode literals keep the low 8 bits, and compare by value.

module Test

@main
func main() -> lang.i64 {
    let a: ExitCode = 7;
    if a.value != 7 { return 1 }
    let wrapped: ExitCode = 261;
    if wrapped.value != 5 { return 2 }
    if not ExitCode.success.isSuccess { return 3 }
    if ExitCode.failure.isSuccess { return 4 }
    if not a.isEqual(to: ExitCode(7)) { return 5 }
    let high: ExitCode = 255;
    if high.value != 255 { return 6 }
    0
}
