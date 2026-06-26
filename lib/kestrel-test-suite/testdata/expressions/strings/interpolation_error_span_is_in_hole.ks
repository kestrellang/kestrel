// test: diagnostics
// stdlib: true
//
// #166 (BUG-30): a diagnostic raised on an expression inside a `\(...)`
// interpolation hole must be anchored at the hole, not at the file start.
// The hole expression is re-lexed/re-parsed against a substring, so its CST
// spans were left relative to that substring (offset 0) and rendered as
// file-absolute — pointing at line 1. This `diagnostics` test pins the span
// by line: if the diagnostic regressed to the file start, the expected-error
// annotation on the offending line below would no longer match.

module Test

func f() -> () {
    let s = "hello";
    let _ = "t1=\(s.bogusMember)"; // ERROR: no member 'bogusMember' on type 'String'
}
