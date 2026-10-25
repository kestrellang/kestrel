// test: execution
// stdlib: true
// expect-exit: 0

// Implicit `it` belongs to the innermost enclosing closure written WITHOUT a
// parameter list. A nested closure that declares its parameters is
// transparent: the `it` inside `{ (y) in y == it }` is the outer `map`
// closure's element. Detection used to stop at every nested closure, so this
// failed with "undefined name 'it'".

module Test

@main
func main() -> lang.i64 {
    let xs = [1, 2];
    let ys = [2, 3];
    let counts = xs.map { ys.filter(where: { (y) in y == it }).count };
    if counts(0) != 0 { return 1; }
    if counts(1) != 1 { return 2; }
    0
}
