// Rosetta Code: Stack
// https://rosettacode.org/wiki/Stack
module Main

// A stack is a thin wrapper over Array's push/pop at the end.
struct Stack[T] {
    var items: Array[T];

    init() { self.items = Array[T](); }

    mutating func push(consuming item: T) { self.items.append(item); }
    mutating func pop() -> T? { self.items.popLast() }
    func peek() -> Optional[&T] { self.items.last }
    var isEmpty: Bool { get { self.items.isEmpty } }
}

@main
func main() {
    var s = Stack[Int64]();
    println("empty: \(s.isEmpty)");
    for i in 1..=3 { s.push(i * 10); }
    println("top: \(s.peek() ?? 0)");
    while let some x = s.pop() {
        println("popped \(x)");
    }
    println("empty: \(s.isEmpty)");
}
