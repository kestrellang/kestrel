// test: diagnostics
// stdlib: false

module Main

struct Factory {
    static func create() -> lang.i64 {
        42
    }

    func build() -> lang.i64 {
        42
    }
}
