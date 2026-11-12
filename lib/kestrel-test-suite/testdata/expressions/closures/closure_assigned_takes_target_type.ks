// test: execution
// stdlib: true
// expect-exit: 0

module Test

// A closure literal assigned to a typed place takes its parameter types from
// the place, as a closure argument takes them from the parameter.
@main
func main() -> lang.i64 {
    let s = "abcd";
    var g: escaping (&String) -> Int64 = { it.bytes.count };
    g = { it.bytes.count + 1 };
    if g(s) != 5 { return 1 }
    // A Dictionary subscript place is `V?`.
    var handlers: [String: escaping (&String) -> Int64] = [:];
    handlers("len") = { it.bytes.count * 2 };
    let h = handlers("len")!;
    if h(s) != 8 { return 2 }
    var list: [escaping (&String) -> Int64] = [g];
    let i: Int64 = 0;
    list(i) = { it.bytes.count + 3 };
    if list(i)(s) != 7 { return 3 }
    0
}
