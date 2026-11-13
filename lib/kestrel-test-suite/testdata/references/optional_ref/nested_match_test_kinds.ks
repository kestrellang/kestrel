// test: execution
// stdlib: true
// expect-stdout: 1 1 4 3\n

// Every decision-tree test kind reads through a lent `Optional[&V]`
// payload without holding the reference across the branch (E497): a Bool
// branch, a string-literal chain, a struct pattern with a nested enum
// switch, and an enum switch on a lent `last()`.
module Test

import std.collections.(Array, Dictionary)

enum Shape {
    case Circle(Int64)
    case Square(Int64)
}

struct Box { var shape: Shape; var on: Bool }

@main
func main() -> lang.i64 {
    var flags: Dictionary[String, Bool] = [:];
    flags.insert("a", true);
    let f = match flags("a") { some true => 1, some false => 2, .None => 3 };
    var names: Dictionary[Int64, String] = [:];
    names.insert(1, "one");
    let n = match names(1) { some "one" => 1, _ => 0 };
    var boxes: Dictionary[Int64, Box] = [:];
    boxes.insert(1, Box(shape: .Square(4), on: true));
    let b = match boxes(1) { some Box { shape: .Square(s), on: true } => s, some _ => -1, .None => -2 };
    var xs = [Shape.Circle(2), Shape.Square(3)];
    let c = match xs.last { some .Square(s) => s, _ => 0 };
    print("\(f) \(n) \(b) \(c)\n");
    0
}
