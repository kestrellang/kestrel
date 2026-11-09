module Main

import std.system.Process

// Each node is a heap allocation (an `indirect` enum case); leaves are the
// empty case. No custom pools, per the benchmark rules.
indirect enum Tree {
    case Node(left: Tree, right: Tree)
    case Empty
}

func make(depth: Int64) -> Tree {
    if depth == 0 { return .Node(left: .Empty, right: .Empty) }
    .Node(left: make(depth - 1), right: make(depth - 1))
}

func check(t: Tree) -> Int64 {
    match t {
        .Node(l, r) => 1 + check(l) + check(r),
        .Empty => 0,
    }
}

@main
func main() {
    let args = Process.arguments();
    let n = if args.count > 1 { Int64(parsing: args(1)) ?? 10 } else { 10 };
    let minDepth: Int64 = 4;
    let maxDepth = if minDepth + 2 > n { minDepth + 2 } else { n };

    let stretch = maxDepth + 1;
    println("stretch tree of depth \(stretch)\t check: \(check(make(stretch)))");

    let longLived = make(maxDepth);

    var d = minDepth;
    while d <= maxDepth {
        let iterations: Int64 = 1 << (maxDepth - d + minDepth);
        var sum: Int64 = 0;
        for _ in 0..<iterations {
            sum = sum + check(make(d));
        }
        println("\(iterations)\t trees of depth \(d)\t check: \(sum)");
        d = d + 2;
    }
    println("long lived tree of depth \(maxDepth)\t check: \(check(longLived))");
}
