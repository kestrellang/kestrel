// test: execution
// stdlib: true
// expect-exit: 0

// `a(0).append(2)` passes the element's `mutating ref` straight to the call.
// Ending that ref must also end its mutable borrow of `a`; it used to stay
// open to scope exit, so a later closure reading `a` (or the hidden closure
// on the right of `or`) hit an OSSA "active mut borrow" ICE.
module Test

@main
func main() -> Int64 {
    var a: [[Int64]] = [[1]];
    a(0).append(2);
    let f = { a.count };
    let b = false or a(0).count != 2;
    if b { return 1; }
    f() - 1
}
