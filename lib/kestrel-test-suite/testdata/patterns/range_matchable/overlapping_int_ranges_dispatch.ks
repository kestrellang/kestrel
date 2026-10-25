// test: execution
// stdlib: true
// expect-exit: 0
//
// F1 — two arms whose integer ranges overlap. Each value must reach the FIRST
// arm that contains it. The decision tree used to keep an arm for any case its
// range merely overlapped, so 12 (outside `0..=10`) was routed to arm 1.

module Test

func classify(n: Int64) -> Int64 {
    match n {
        0..=10 => 1,
        5..=15 => 2,
        _ => 0
    }
}

// A literal inside a later range must not claim the whole range.
func literalFirst(n: Int64) -> Int64 {
    match n {
        5 => 2,
        0..=10 => 1,
        _ => 0
    }
}

// An open range overlapping a later one.
func openRange(n: Int64) -> Int64 {
    match n {
        ..<0 => 1,
        ..<5 => 2,
        _ => 0
    }
}

@main
func main() -> lang.i32 {
    if classify(3) != 1 { return 1 }
    if classify(7) != 1 { return 2 }
    if classify(12) != 2 { return 3 }
    if classify(20) != 0 { return 4 }
    if literalFirst(5) != 2 { return 5 }
    if literalFirst(3) != 1 { return 6 }
    if literalFirst(11) != 0 { return 7 }
    if openRange(-3) != 1 { return 8 }
    if openRange(3) != 2 { return 9 }
    if openRange(9) != 0 { return 10 }
    0
}
