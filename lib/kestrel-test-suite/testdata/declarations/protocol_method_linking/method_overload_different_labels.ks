// test: diagnostics
// stdlib: false
module Test

protocol Printer {
    func print(value value: lang.i64)
    func print(text text: lang.str)
}
struct Console: Printer {
    func print(value value: lang.i64) { }
    func print(text text: lang.str) { }
}
