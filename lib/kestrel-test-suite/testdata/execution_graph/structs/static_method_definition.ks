// test: diagnostics
// stdlib: false

module Main

struct Math {
    static func add(a: lang.i64, b: lang.i64) -> lang.i64 {
        lang.i64_add(a, b)
    }
}
