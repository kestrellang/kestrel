// Rosetta Code: Queue/Definition
// https://rosettacode.org/wiki/Queue/Definition
module Main

import std.collections.Deque

// FIFO queue on top of the standard ring-buffer Deque.
struct Queue[T] {
    var items: Deque[T];

    init() { self.items = Deque[T](); }

    mutating func push(consuming item: T) { self.items.pushBack(item); }
    mutating func pop() -> T? { self.items.popFront() }
    var isEmpty: Bool { get { self.items.isEmpty } }
}

@main
func main() {
    var q = Queue[String]();
    println("empty: \(q.isEmpty)");
    q.push("first");
    q.push("second");
    q.push("third");
    println("pop: \(q.pop() ?? "-")");
    q.push("fourth");
    while let .Some(x) = q.pop() {
        println("pop: \(x)");
    }
    println("empty: \(q.isEmpty)");
    println("pop on empty: \(q.pop() ?? "nothing")");
}
