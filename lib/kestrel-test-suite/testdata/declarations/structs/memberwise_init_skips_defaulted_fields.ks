// test: execution
// stdlib: true
// expect-stdout: 1 3 x 5 3 y\nbox 1 [t] 1 [q]\n

// A memberwise init may leave out fields that have a default: `S(a: 1)`
// fills `b` and `c` from `= 3` and `= "x"`. Given arguments still follow
// declaration order, and a default is only evaluated for a field left out.
module Test

struct S {
    var a: Int64
    var b: Int64 = 3
    var c: String = "x"
}

struct Box[T] {
    var item: T
    var count: Int64 = 1
    var tags: [String] = []
}

@main
func main() -> Int64 {
    let s = S(a: 1);
    let t = S(a: 1, b: 5);
    let u = S(a: 2, c: "y");
    println("\(s.a) \(s.b) \(s.c) \(t.b) \(u.b) \(u.c)");
    var b = Box(item: "box");
    b.tags.append("t");
    let c = Box(item: 2, tags: ["q"]);
    println("\(b.item) \(b.count) \(b.tags) \(c.count) \(c.tags)");
    0
}
