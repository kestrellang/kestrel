// test: execution
// stdlib: true
// expect-stdout: gpos=6 pos=4 count=13\n

// Field writes through a global or a `static var` land in the stored value,
// and a static computed property's `+=` goes through its setter (#140).
// All three used to write a loaded copy and drop it.
module Test

struct Pos { var x: Int64 }

var gpos: Pos = Pos(x: 0);

struct S {
    static var count: Int64 = 0
    static var pos: Pos = Pos(x: 0)
    static var calc: Int64 { get { S.count } set { S.count = newValue; } }
}

@main
func main() -> Int64 {
    gpos.x = 6;
    S.pos.x = 4;
    S.count += 3;
    S.calc += 10;
    print("gpos=\(gpos.x) pos=\(S.pos.x) count=\(S.count)\n");
    0
}
