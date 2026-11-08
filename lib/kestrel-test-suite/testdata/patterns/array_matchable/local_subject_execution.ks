// test: execution
// stdlib: true
// expect-exit: 0
//
// Array patterns on a LOCAL array (not a parameter). The owned local takes
// the place-mode / moveout path, which used to treat a prefix element
// `Index(i)` as a struct field of `Array` and destructure its storage —
// an OSSA ICE ("type mismatch" / "operand used but never defined").

module Test

func head() -> Int64 {
    let xs = [5, 3, 9];
    match xs {
        [a, ..] => a,
        _ => -1
    }
}

func restCount() -> Int64 {
    let xs = [5, 3, 9];
    match xs {
        [_, ..rest] => rest.count,
        _ => -1
    }
}

func firstString() -> Int64 {
    let xs = ["a", "bb", "ccc"];
    match xs {
        [a, ..rest] => if a == "a" { rest.count } else { -2 },
        _ => -1
    }
}

@main
func main() -> lang.i64 {
    if head() != 5 { return 1 }
    if restCount() != 2 { return 2 }
    if firstString() != 2 { return 3 }
    0
}
