// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Payload: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

indirect enum Tree: not Copyable {
    case Leaf(value: Payload)
    case Node(left: Tree, right: Tree)
}

func test() {
    let tree = Tree.Node(
        left: Tree.Leaf(value: Payload(id: 1)),
        right: Tree.Node(
            left: Tree.Leaf(value: Payload(id: 2)),
            right: Tree.Leaf(value: Payload(id: 3))
        )
    );
    // Recursively deinits all 3 Payloads
}

func main() -> lang.i64 {
    test();
    if deinit_count != 3 { return 1; }
    0
}
