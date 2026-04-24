// test: diagnostics
// stdlib: false
module Test

struct Container[T] { let value: T }
protocol Printable { func print() }
extend Container[lang.i64]: Printable {
    func print() { }
}
func usePrintable(p: Printable) { p.print(); }
func main() {
    let c = Container(value: 42);
    usePrintable(c);
}
