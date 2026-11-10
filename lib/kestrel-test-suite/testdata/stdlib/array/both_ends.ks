// test: execution
// stdlib: true
// expect-stdout: [0, 1, 2, 3] Some(3) Some(0) [1, 2]\nSome(9) None\n

// Array and Deque share one set of end verbs: `append` / `popLast` at the
// back, `prepend` / `popFirst` at the front.
module Test

import std.collections.(Deque)

@main
func main() -> Int64 {
    var a = [1, 2];
    a.append(3);
    a.prepend(0);
    let shown = "\(a)";
    let back = a.popLast();
    let front = a.popFirst();
    println("\(shown) \(back) \(front) \(a)");
    var d = Deque[Int64]();
    d.prepend(9);
    let only = d.popLast();
    println("\(only) \(d.popFirst())");
    0
}
