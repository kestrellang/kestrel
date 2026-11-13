// test: execution
// stdlib: true
// expect-stdout: 7.5 2\n

module Test

// `T.Addable.Output` names `Addable`'s `Output` among the several a bound
// type has (`Subtractable` declares one too), as `type Addable.Output = …`
// does in a conformance. Before, the qualified form was dropped and
// `a + b` stayed `T.Output`.
func addThenSubtract[T](a: T, b: T, c: T) -> T
    where T: Addable, T: Subtractable, T: Copyable,
          T.Addable.Output = T, T.Subtractable.Output = T {
    a + b - c
}

@main
func main() {
    let x: Int32 = 5;
    println("\(addThenSubtract(3.0, 5.5, 1.0)) \(addThenSubtract(x, 1, 4))");
}
