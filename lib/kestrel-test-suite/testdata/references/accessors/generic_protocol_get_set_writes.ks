// test: execution
// stdlib: true
// expect-stdout: n=9 pos.x=5 i0=9 count=2\n

// Writes through `{ get set }` protocol requirements in a generic body. A
// requirement has no setter of its own, so every read-modify-write must call
// the `name.set` witness after modifying the gotten value. Only a plain
// `x.n = 4` used to work; these were dropped, and `append` crashed.
module Test

struct Pos { var x: Int64 }

struct T { var items: [Int64]  var pos: Pos  var n: Int64 }

protocol HasStuff {
    var items: [Int64] { get set }
    var pos: Pos { get set }
    var n: Int64 { get set }
}

extend T: HasStuff {}

func g[X](mutating x: X) where X: HasStuff {
    x.n -= 1;
    x.pos.x = 5;
    x.items(0) = 9;
    x.items.append(3);
}

@main
func main() -> Int64 {
    var t = T(items: [1], pos: Pos(x: 0), n: 10);
    g(t);
    print("n=\(t.n) pos.x=\(t.pos.x) i0=\(t.items(0)) count=\(t.items.count)\n");
    0
}
