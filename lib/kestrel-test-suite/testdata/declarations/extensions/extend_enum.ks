// test: diagnostics
// stdlib: false
module Test
enum Color {
    case Red
    case Green
    case Blue
}
extend Color {
    func isRed() -> lang.i1 { return true; }
}
