// test: execution
// stdlib: true
// expect-stdout: 45\n

// G29 control: the same function shapes as the two E479 tests, but only
// `HasOutA` declares `Out` (`HasOther` does not), so `T.Out` names exactly
// one associated type. Both the equality and the projection bound are kept,
// and the program runs.

module Test

protocol HasOutA { type Out; func outA() -> Out }
protocol HasOther { func other() -> Int64 }
protocol Show { func show() -> Int64 }

func viaEquality[T](t: T) -> Int64 where T: HasOutA, T: HasOther, T.Out = Int64 {
    t.outA() + t.other()
}

func viaBound[T](t: T) -> Int64 where T: HasOutA, T: HasOther, T.Out: Show {
    t.outA().show()
}

struct Num: Show {
    var n: Int64;
    func show() -> Int64 { self.n }
}

struct Gen: HasOutA, HasOther {
    type Out = Int64;
    func outA() -> Int64 { 40 }
    func other() -> Int64 { 2 }
}

struct GenNum: HasOutA, HasOther {
    type Out = Num;
    func outA() -> Num { Num(n: 3) }
    func other() -> Int64 { 0 }
}

@main
func main() -> lang.i32 {
    print(viaEquality(Gen()) + viaBound(GenNum()));
    0
}
