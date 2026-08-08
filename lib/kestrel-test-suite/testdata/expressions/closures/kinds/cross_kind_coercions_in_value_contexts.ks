// test: execution
// stdlib: true
// expect-exit: 0

// Cross-kind replay must happen before values are stored or returned, not only
// while preparing direct call arguments.
module Test

import std.numeric.(Int64)

func adapt(f: () -> Int64) -> mutating () -> Int64 { f }

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { count = count + 1; count }
}

@main
func main() -> lang.i64 {
    var base = 4;
    let normal: () -> Int64 = { base };
    var mutatingView: mutating () -> Int64 = adapt(normal);
    if mutatingView() != 4 { return 1 }
    base = 5;
    if mutatingView() != 5 { return 2 }

    let shared: escaping () -> Int64 = makeCounter(10);
    let normalView: () -> Int64 = shared;
    if normalView() != 11 { return 3 }
    if shared() != 12 { return 4 }

    let oneShot: consuming () -> Int64 = shared;
    if oneShot() != 13 { return 5 }
    if shared() != 14 { return 6 }

    0
}
