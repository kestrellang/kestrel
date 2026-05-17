// test: diagnostics
// stdlib: true

module Test

func make_curried_add() -> (std.numeric.Int64) -> (std.numeric.Int64) -> std.numeric.Int64 {
    { (a) in { (b) in a + b } } // ERROR: cannot return a closure that captures variables
}

func main() -> lang.i64 {
    let curried = make_curried_add();
    let add20 = curried(20);
    if add20(22) != 42 { return 1 }
    0
}
