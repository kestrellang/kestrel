// test: execution
// stdlib: true
// backends: cranelift,llvm

// `if let .Some(&mutating v) = dict(key)` mutates the stored value in
// place (`modify(key:body:)` was retired for it); a missing key runs no
// body.
module Test

import std.collections.(Dictionary)
import std.numeric.(Int64)
import std.text.(String)

@main
func main() -> lang.i64 {
    var dict = Dictionary[String, Int64]();
    dict.insert("a", 1);
    dict.insert("b", 20);

    // Mutate in place; closure result comes back through the Optional.
    var r1: Int64? = .None;
    if let .Some(&mutating v) = dict("a") {
        v = v + 41;
        r1 = .Some(v);
    }
    if r1 != .Some(42) { return 1 }
    if dict("a").unwrap() != 42 { return 2 }

    // Closure result type R may differ from V.
    var r2: Bool? = .None;
    if let .Some(&mutating v) = dict("b") {
        let old = v;
        v = v * 2;
        r2 = .Some(old > 10);
    }
    if r2 != .Some(true) { return 3 }
    if dict("b").unwrap() != 40 { return 4 }

    // Missing key: None, value space untouched.
    var r3: Int64? = .None;
    if let .Some(&mutating v) = dict("zzz") {
        v = -999;
        r3 = .Some(v);
    }
    if r3 != .None { return 5 }
    if dict.count != 2 { return 6 }

    // COW: a sibling copy taken before modify is unaffected.
    let snapshot = dict;
    if let .Some(&mutating v) = dict("a") { v = 0; }
    if dict("a").unwrap() != 0 { return 7 }
    if snapshot("a").unwrap() != 42 { return 8 }

    0
}
