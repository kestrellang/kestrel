// test: execution
// stdlib: true

module Test

// Deque's subscript is part of its public API: code outside std can read
// and write elements by index, including across the ring buffer's wrap point.
@main
func main() -> lang.i64 {
    var d = std.collections.Deque[std.numeric.Int64]();
    d.pushBack(20);
    d.pushBack(30);
    d.pushFront(10);
    if d(0) != 10 { return 1 }
    if d(1) != 20 { return 2 }
    if d(2) != 30 { return 3 }
    d(1) = 99;
    if d(1) != 99 { return 4 }
    0
}
