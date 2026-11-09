// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: apple banana cherry date elder fig grape\n

// A heap of heap-owning elements: sifting moves them (no clones, no drops),
// so every string comes out intact and in order.
module Test

import std.collections.(Heap)

@main
func main() {
    var h = Heap[String]();
    for w in ["fig", "cherry", "grape", "apple", "elder", "banana", "date"] { h.push(w); }
    var first = true;
    while let .Some(w) = h.pop() {
        if not first { print(" ") }
        print(w);
        first = false;
    }
    print("\n");
}
