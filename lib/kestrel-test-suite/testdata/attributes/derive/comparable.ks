// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// `@derive(Comparable)`: fields compare lexicographically, the first one
// that isn't `.Equal` deciding; enum cases compare by declaration order
// first, then by payload. `<`, `>=`, ... come from `Comparable`.

module Main

import std.numeric.(Int64)

@derive(Equatable, Comparable)
struct Version {
    var major: Int64
    var minor: Int64
    var patch: Int64
}

@derive(Equatable, Comparable)
enum Priority {
    case Low
    case Medium(Int64)
    case High(Int64, Int64)
}

@main
func main() -> Int64 {
    let v1 = Version(major: 1, minor: 2, patch: 0);
    let v2 = Version(major: 1, minor: 10, patch: 0);
    let v3 = Version(major: 2, minor: 0, patch: 0);
    if v1 >= v2 { return 1 }
    if v2 >= v3 { return 2 }
    if v3 <= v1 { return 3 }
    if v1.compare(v1) != Ordering.Equal { return 4 }
    if Version(major: 1, minor: 2, patch: 3) <= Version(major: 1, minor: 2, patch: 1) { return 5 }
    if Priority.Low >= Priority.Medium(0) { return 6 }
    if Priority.Medium(9) >= Priority.High(0, 0) { return 7 }
    if Priority.Medium(1) >= Priority.Medium(2) { return 8 }
    if Priority.High(1, 5) >= Priority.High(2, 0) { return 9 }
    if Priority.High(1, 5) <= Priority.High(1, 4) { return 10 }
    if Priority.Low.compare(Priority.Low) != Ordering.Equal { return 11 }
    0
}
