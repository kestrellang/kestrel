// test: diagnostics
// stdlib: false
module Test

struct Container[T] { let value: T }
protocol Printable { func print() }
extend Container[lang.i64]: Printable {
    func print() { }
}
// A bare protocol is not a value type (E144); take the conformer generically.
func usePrintable[P](p: P) where P: Printable { p.print(); }
func main() {
    let c = Container(value: 42);
    usePrintable(c);
}
