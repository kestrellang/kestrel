// test: execution
// stdlib: true
// expect-exit: 0

// Pins the kind prefix in a *return* type. Only the owning kinds may appear
// there — `escaping` (shared, multi-call, mutates its own snapshot) and
// `consuming` (unique, one-shot). Returning a capturing closure is exactly the
// behavior change #4 in docs/design/closures.md ("Behavior Changes from Today").
module Test

import std.numeric.(Int64)

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { count = count + 1; count }
}

func makeOneShot(value: Int64) -> consuming () -> Int64 {
    { value }
}

@main
func main() -> lang.i64 {
    let next = makeCounter(10);
    if next() != 11 { return 1 }
    if next() != 12 { return 2 }

    let once = makeOneShot(5);
    if once() != 5 { return 3 }

    0
}
