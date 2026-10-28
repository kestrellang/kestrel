// test: execution
// stdlib: true
// backends: cranelift,llvm

// get→op→set writeback (the permanent fallback half of stage 1.5): a
// computed get/set subscript fabricates its elements, so RMW copies the
// element out through `get`, mutates, and writes back through `set`.
// Also pins the flagship stdlib case — `arr(i) += v` through the Slice
// extension's get/set subscript (a WITNESS setter) — and a computed
// get/set property RMW.
module Test

import std.numeric.(Int64)

func pow10(n: Int64) -> Int64 {
    var r: Int64 = 1;
    var i: Int64 = 0;
    while i < n {
        r = r * 10;
        i = i + 1;
    }
    r
}

struct Packed {
    var bits: Int64

    subscript(at index: Int64) -> Int64 {
        get { (self.bits / pow10(index)) % 10 }
        set {
            let old = (self.bits / pow10(index)) % 10;
            self.bits = self.bits + (newValue - old) * pow10(index);
        }
    }

    var ones: Int64 {
        get { self.bits % 10 }
        set { self.bits = self.bits - (self.bits % 10) + newValue; }
    }
}

struct Score {
    var points: Int64
    mutating func double() { self.points = self.points * 2; }
}

struct PackedScore {
    var stored: Score
    subscript(at index: Int64) -> Score {
        get { self.stored }
        set { self.stored = newValue; }
    }
}

@main
func main() -> lang.i64 {
    var p = Packed(bits: 345);
    if p(at: 0) != 5 { return 1; }

    // RMW through get → op → set.
    p(at: 0) += 2;
    if p.bits != 347 { return 2; }
    p(at: 2) -= 1;
    if p.bits != 247 { return 3; }

    // Computed property writeback.
    p.ones += 1;
    if p.bits != 248 { return 4; }

    // Mutating METHOD through writeback — the mutation persists (the
    // stage-1.5 behavior change from mutate-a-discarded-temp).
    var ps = PackedScore(stored: Score(points: 7));
    ps(at: 0).double();
    if ps.stored.points != 14 { return 5; }

    // Flagship: stdlib Array unlabeled subscript (Slice extension,
    // witness get/set) now supports compound assignment.
    var arr = [10, 20, 30];
    arr(1) += 5;
    if arr(1) != 25 { return 6; }
    arr(0) *= 3;
    if arr(0) != 30 { return 7; }
    if arr(2) != 30 { return 8; }
    0
}
