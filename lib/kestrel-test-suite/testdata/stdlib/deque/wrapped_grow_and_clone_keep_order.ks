// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 0 1 2 3 4 5 6 7 8 9\n0 1 2 3 4 5 6 7 8 9 99\na b c d e f\n

// A deque whose ring has wrapped keeps its order when it grows and when a
// copy is forked, for byte-copied elements (copied as blocks) and for
// elements that clone one by one.
module Test

import std.collections.(Deque)

func show(d: Deque[Int64]) {
    var first = true;
    for x in d { if not first { print(" ") } print("\(x)"); first = false; }
    print("\n");
}

@main
func main() {
    // Capacity 4: pushes at the front and back wrap the ring before it grows.
    var d = Deque[Int64](capacity: 4);
    d.pushBack(2); d.pushBack(3); d.pushFront(1); d.pushFront(0);
    for i in 4..<10 { d.pushBack(i); }
    show(d);

    var w = Deque[Int64](capacity: 8);
    for i in 4..<10 { w.pushBack(i); }
    for i in 0..<4 { w.pushFront(3 - i); }
    var copy = w;
    copy.pushBack(99);
    show(copy);

    var s = Deque[String](capacity: 4);
    s.pushBack("c"); s.pushBack("d"); s.pushFront("b"); s.pushFront("a");
    var s2 = s;
    s2.pushBack("e"); s2.pushBack("f");
    var first = true;
    for x in s2 { if not first { print(" ") } print(x); first = false; }
    print("\n");
}
