// test: execution
// stdlib: true
// backends: cranelift,llvm

// Dictionary.modify(key, with:) — stage 1.5 item-4 interim: in-place value
// mutation via bucket writeback, returning the closure's result, or None
// (body not invoked) for a missing key. Upgrades to true in-place access
// later (Optional[&T]) with no API change.
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
    let r1 = dict.modify("a") { (mutating v) in
        v = v + 41;
        v
    };
    if r1 != .Some(42) { return 1 }
    if dict("a").unwrap() != 42 { return 2 }

    // Closure result type R may differ from V.
    let r2 = dict.modify("b") { (mutating v) in
        let old = v;
        v = v * 2;
        old > 10
    };
    if r2 != .Some(true) { return 3 }
    if dict("b").unwrap() != 40 { return 4 }

    // Missing key: None, value space untouched.
    let r3 = dict.modify("zzz") { (mutating v) in
        v = -999;
        v
    };
    if r3 != .None { return 5 }
    if dict.count != 2 { return 6 }

    // COW: a sibling copy taken before modify is unaffected.
    let snapshot = dict;
    dict.modify("a") { (mutating v) in v = 0 };
    if dict("a").unwrap() != 0 { return 7 }
    if snapshot("a").unwrap() != 42 { return 8 }

    0
}
