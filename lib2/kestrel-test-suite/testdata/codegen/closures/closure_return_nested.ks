// test: diagnostics
// stdlib: true

module Test

func make_curried_add() -> (std.num.Int64) -> (std.num.Int64) -> std.num.Int64 {
    { (a) in { (b) in a + b } }
}

func main() -> lang.i64 {
    let curried = make_curried_add();
    let add20 = curried(20);
    if add20(22) != 42 { return 1 }
    0
}
