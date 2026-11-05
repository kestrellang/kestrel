// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: swap 0\nreverse 0\ninsert 0\npop 0\nremoveSubrange 0\nreplaceSubrange shrink 0\nreplaceSubrange grow 0\ndedup 0\nrotate 0\nsort 0\n

// In-place Array algorithms move elements between slots. Moving with
// `read()` (a copy) instead of `take()` cloned each moved element and never
// dropped the original, and removeSubrange / replaceSubrange overwrote the
// removed elements without dropping them. Each line prints how many values
// are still alive after the array goes away: it must be 0.
module Test

struct Live { static var count: Int64 = 0 }

struct Tracked: Cloneable, Equatable {
    var n: Int64;
    init(n n: Int64) { Live.count += 1; self.n = n }
    func clone() -> Tracked { Tracked(n: self.n) }
    func isEqual(to other: Tracked) -> Bool { self.n == other.n }
    deinit { Live.count -= 1 }
}

func make(ns: Array[Int64]) -> Array[Tracked] {
    var out = Array[Tracked]();
    for n in ns { out.append(Tracked(n: n)); }
    out
}

func report(label: String) {
    println("\(label) \(Live.count)");
    Live.count = 0;
}

func swapCase() { var xs = make([1, 2]); xs.swap(at: 0, with: 1); }
func reverseCase() { var xs = make([1, 2, 3]); xs.reverse(); }
func insertCase() { var xs = make([1, 2, 3]); xs.insert(Tracked(n: 0), at: 0); }
func popCase() { var xs = make([1, 2, 3]); let _ = xs.pop(); }
func removeSubrangeCase() { var xs = make([1, 2, 3, 4]); xs.removeSubrange(1..<3); }
func replaceShrinkCase() { var xs = make([1, 2, 3, 4]); xs.replaceSubrange(1..<3, with: make([9])); }
func replaceGrowCase() { var xs = make([1, 2, 3, 4]); xs.replaceSubrange(1..<2, with: make([8, 9, 10])); }
func dedupCase() { var xs = make([1, 1, 2, 2, 3, 1]); xs.dedup(); }
func rotateCase() { var xs = make([1, 2, 3, 4, 5]); xs.rotate(by: 2); }
func sortCase() { var xs = make([3, 1, 2, 5, 4]); xs.sort(by: { (a, b) in a.n < b.n }); }

@main
func main() -> lang.i64 {
    swapCase(); report("swap");
    reverseCase(); report("reverse");
    insertCase(); report("insert");
    popCase(); report("pop");
    removeSubrangeCase(); report("removeSubrange");
    replaceShrinkCase(); report("replaceSubrange shrink");
    replaceGrowCase(); report("replaceSubrange grow");
    dedupCase(); report("dedup");
    rotateCase(); report("rotate");
    sortCase(); report("sort");
    0
}
