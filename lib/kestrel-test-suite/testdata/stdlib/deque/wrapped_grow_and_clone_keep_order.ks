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
    d.append(2); d.append(3); d.prepend(1); d.prepend(0);
    for i in 4..<10 { d.append(i); }
    show(d);

    var w = Deque[Int64](capacity: 8);
    for i in 4..<10 { w.append(i); }
    for i in 0..<4 { w.prepend(3 - i); }
    var copy = w;
    copy.append(99);
    show(copy);

    var s = Deque[String](capacity: 4);
    s.append("c"); s.append("d"); s.prepend("b"); s.prepend("a");
    var s2 = s;
    s2.append("e"); s2.append("f");
    var first = true;
    for x in s2 { if not first { print(" ") } print(x); first = false; }
    print("\n");
}
