module Main

import std.system.Process
import std.memory.(Pointer, SystemAllocator, Layout)

// One malloc per node and one free per node, like the C program: a node is
// two child pointers (16 bytes), and leaves have null children. Raw
// pointers keep the node at 16 bytes; the `indirect enum` version costs 48.
struct Node {
    var left: Pointer[Node]
    var right: Pointer[Node]
}

func alloc(left: Pointer[Node], right: Pointer[Node]) -> Pointer[Node] {
    var a = SystemAllocator();
    match a.allocate(Layout.of[Node]()) {
        .Some(raw) => {
            let p = raw.cast[Node]();
            unsafe p.write(Node(left: left, right: right));
            p
        },
        .None => fatalError("out of memory"),
    }
}

func make(depth: Int64) -> Pointer[Node] {
    if depth == 0 { return alloc(Pointer[Node].nullPointer(), Pointer[Node].nullPointer()) }
    alloc(make(depth - 1), make(depth - 1))
}

func check(p: Pointer[Node]) -> Int64 {
    let node = unsafe p.read();
    if node.left.isNull { return 1 }
    1 + check(node.left) + check(node.right)
}

func release(p: Pointer[Node]) {
    let node = unsafe p.read();
    if not node.left.isNull {
        release(node.left);
        release(node.right);
    }
    var a = SystemAllocator();
    unsafe a.deallocate(p.asRaw(), Layout.of[Node]());
}

@main
func main() {
    let args = Process.arguments();
    let n = if args.count > 1 { Int64(parsing: args(1)) ?? 10 } else { 10 };
    let minDepth: Int64 = 4;
    let maxDepth = if minDepth + 2 > n { minDepth + 2 } else { n };

    let stretch = maxDepth + 1;
    let st = make(stretch);
    println("stretch tree of depth \(stretch)\t check: \(check(st))");
    release(st);

    let longLived = make(maxDepth);

    var d = minDepth;
    while d <= maxDepth {
        let iterations: Int64 = 1 << (maxDepth - d + minDepth);
        var sum: Int64 = 0;
        for _ in 0..<iterations {
            let t = make(d);
            sum = sum + check(t);
            release(t);
        }
        println("\(iterations)\t trees of depth \(d)\t check: \(sum)");
        d = d + 2;
    }
    println("long lived tree of depth \(maxDepth)\t check: \(check(longLived))");
    release(longLived);
}
