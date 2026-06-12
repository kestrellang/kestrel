// test: diagnostics
// stdlib: true

// The v1 stdlib forwarding surface: refs forward Equatable and Comparable
// (both mutabilities, via core/ref.ks). Everything else still cleanly
// rejects — Hashable here pins that the acceptance is extension-driven,
// not a blanket "refs conform as their pointee" rule.
module Test

func needEq[X]() -> Int64 where X: not Static, X: Equatable { 1 }
func needOrd[X]() -> Int64 where X: not Static, X: Comparable { 2 }
func needHash[X]() -> Int64 where X: not Static, X: Hashable { 3 }

@main
func main() {
    let a = needEq[&Int64]();
    let b = needEq[&mutating Int64]();
    let c = needOrd[&Int64]();
    let d = needOrd[&mutating Int64]();
    let e = needHash[&Int64](); // ERROR: !: Hashable
}
