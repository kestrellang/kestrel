// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Numeric patterns may be negative: literals and range bounds take a
// leading `-` (`-1`, `-5..=5`, `-10..<-1`, `..<-3`, `-3..`).

module Test

func f(x: Int64) -> Int64 {
    match x {
        -1 => 1,
        -10..<-3 => 2,
        -3..=3 => 3,
        _ => 4
    }
}

func g(x: Int64) -> Int64 {
    match x {
        ..<-3 => 1,
        -3.. => 2
    }
}

@main
func main() -> lang.i64 {
    if f(-1) != 1 { return 1 }
    if f(-5) != 2 { return 2 }
    if f(-3) != 3 { return 3 }
    if f(2) != 3 { return 4 }
    if f(-11) != 4 { return 5 }
    if g(-4) != 1 { return 6 }
    if g(-3) != 2 { return 7 }
    if g(100) != 2 { return 8 }
    0
}
