// test: diagnostics
// stdlib: false

module Main

struct Factory {
    static func defaultValue() -> lang.i64 { 0 }
}

struct MathUtils {
    static func max(a: lang.i64, b: lang.i64) -> lang.i64 { 42 }
    static func min(a: lang.i64, b: lang.i64) -> lang.i64 { 0 }
}

func test() -> lang.i64 {
    MathUtils.max(10, 20)
}
