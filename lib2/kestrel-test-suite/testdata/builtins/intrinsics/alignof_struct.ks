// test: diagnostics
// stdlib: false

module Test
struct Data {
    var a: lang.i8
    var b: lang.i64
}
func alignOfData() -> lang.i64 {
    lang.alignof[Data]()
}
