// test: diagnostics
// stdlib: false

// Two overloads of `adding`: one with all-defaulted labeled params, one taking
// `period`. Calling with a skipped leading defaulted label (`months:`) must
// still select the first overload. Previously overload selection matched
// argument labels positionally, so neither candidate matched and it reported a
// spurious E100 "no member 'adding'". See kestrel-ast-builder/src/arg_binding.rs.
module Main

struct Calc {
    public init() {}
    public func adding(years y: lang.i64 = 0, months m: lang.i64 = 0, days d: lang.i64 = 0) -> lang.i64 { y }
    public func adding(period p: lang.i64) -> lang.i64 { p }
}

func test() -> lang.i64 {
    let c = Calc();
    c.adding(months: 1, days: 10)
}
