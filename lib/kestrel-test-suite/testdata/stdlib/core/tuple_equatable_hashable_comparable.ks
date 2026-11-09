// test: execution
// stdlib: true
// expect-exit: 0

module Main

// Tuples of 2 to 4 elements are Equatable, Hashable and Comparable when
// their elements are, so they work as dictionary keys and sort keys.

@main
func main() -> Int64 {
    var seen: [(Int64, Int64): Bool] = [:];
    seen.insert((1, 2), true);
    seen.insert((1, 2), false);
    seen.insert((2, 1), true);
    if seen.count != 2 { return 1 }
    if seen((1, 2)) != .Some(false) { return 2 }

    if (1, 2) != (1, 2) { return 3 }
    if not ((1, 2) < (1, 3)) { return 4 }
    if not ((1, "b", 3) > (1, "a", 9)) { return 5 }

    var points = [(3, 1), (1, 5), (1, 2)];
    points.sort();
    if points(0) != (1, 2) or points(2) != (3, 1) { return 6 }

    let wide: [(String, Int64, Bool, Char): Int64] = [("a", 1, true, 'x'): 7];
    if (wide(("a", 1, true, 'x')) ?? 0) != 7 { return 7 }
    0
}
