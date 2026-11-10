// test: execution
// stdlib: true

// `Deque.asSlices()` lends its two contiguous runs as `&Slice[T]` in place:
// empty, non-wrapping, and wrapped ring buffers.
module Test

import std.collections.(Deque, Slice)

func total(s: &Slice[Int64]) -> Int64 {
    var sum = 0;
    for v in s { sum = sum + v; }
    sum
}

@main
func main() {
    var d = Deque[Int64](capacity: 4);
    let (e0, e1) = d.asSlices();
    if e0.count + e1.count != 0 { fatalError("empty deque lent elements") }

    d.append(1);
    d.append(2);
    d.append(3);
    let (a, b) = d.asSlices();
    if a.count != 3 or b.count != 0 or a(2) != 3 { fatalError("non-wrapping split broke") }

    // Pushing at the front wraps the head to the end of the buffer.
    d.prepend(0);
    let (front, back) = d.asSlices();
    if front.count + back.count != 4 { fatalError("wrapped split lost elements") }
    if front(0) != 0 { fatalError("wrapped front does not start at the head") }
    print("\(total(front) + total(back))");
}

// CHECK: 6
