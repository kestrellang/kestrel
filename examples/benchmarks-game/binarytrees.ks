module Main

import std.system.Process
import std.memory.(ArenaAllocator, Layout, Pointer)

// Nodes come from std's ArenaAllocator: a node is two child pointers
// (16 bytes), leaves have null children, and a whole tree is freed at once
// with `reset()` instead of one free per node.
struct Node {
    var left: Pointer[Node]
    var right: Pointer[Node]
}

func node(mutating arena: ArenaAllocator, left: Pointer[Node], right: Pointer[Node]) -> Pointer[Node] {
    match arena.allocate(Layout.of[Node]()) {
        .Some(raw) => {
            let p = raw.cast[Node]();
            unsafe p.write(Node(left: left, right: right));
            p
        },
        .None => fatalError("out of memory"),
    }
}

func make(mutating arena: ArenaAllocator, depth: Int64) -> Pointer[Node] {
    if depth == 0 { return node(arena, Pointer[Node].nullPointer(), Pointer[Node].nullPointer()) }
    let l = make(arena, depth - 1);
    let r = make(arena, depth - 1);
    node(arena, l, r)
}

func check(p: Pointer[Node]) -> Int64 {
    let n = unsafe p.read();
    if n.left.isNull { return 1 }
    1 + check(n.left) + check(n.right)
}

@main
func main() {
    let args = Process.arguments();
    let n = if args.count > 1 { Int64(parsing: args(1)) ?? 10 } else { 10 };
    let minDepth: Int64 = 4;
    let maxDepth = if minDepth + 2 > n { minDepth + 2 } else { n };

    // The stretch and per-iteration trees share one arena, reset after each.
    var scratch = ArenaAllocator(chunkSize: 1 << 20);
    let stretch = maxDepth + 1;
    let st = make(scratch, stretch);
    println("stretch tree of depth \(stretch)\t check: \(check(st))");
    scratch.reset();

    var longArena = ArenaAllocator(chunkSize: 1 << 20);
    let longLived = make(longArena, maxDepth);

    var d = minDepth;
    while d <= maxDepth {
        let iterations: Int64 = 1 << (maxDepth - d + minDepth);
        var sum: Int64 = 0;
        for _ in 0..<iterations {
            let t = make(scratch, d);
            sum = sum + check(t);
            scratch.reset();
        }
        println("\(iterations)\t trees of depth \(d)\t check: \(sum)");
        d = d + 2;
    }
    println("long lived tree of depth \(maxDepth)\t check: \(check(longLived))");
    longArena.release();
    scratch.release();
}
