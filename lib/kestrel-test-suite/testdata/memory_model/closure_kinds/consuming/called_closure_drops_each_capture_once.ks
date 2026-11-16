// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: dropped 2\ndropped 1\nmoved 1\nafter call 2\nescaping 3 3\ndropped 5\ndropped 4\nafter drop 2\n

// A consuming closure's box holds its environment until the one call takes
// it out (std.memory.box_bindings: `ConsumingClosureBox`). Every capture
// drops exactly once: one the body moves out drops where it ends up, one it
// leaves drops at the end of the call, and dropping the called closure
// afterwards drops nothing. An escaping closure passed as a consuming one
// keeps its shared environment, and is not used up by the call.
module Test

import std.numeric.Int64
import std.streams.println

public var drops: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    func peek() -> Int64 { self.id }
    deinit {
        drops = drops + 1;
        println("dropped \(self.id)");
    }
}

func run(consuming f: consuming () -> Res) -> Int64 {
    let r = f();
    let id = r.id;
    id
}

func runInt(consuming f: consuming () -> Int64) -> Int64 { f() }

func counter(n: Int64) -> escaping () -> Int64 {
    { () in n }
}

// Both captures drop at the end of the call; dropping `f` after it adds none.
func callThenDrop() {
    let a = Res(id: 4);
    let b = Res(id: 5);
    let f: consuming () -> () = { () in let _ = a.peek(); let _ = b.peek(); };
    f();
}

@main
func main() -> lang.i64 {
    let moved = Res(id: 1);
    let kept = Res(id: 2);
    // `moved` comes back out of the body; `kept` stays in the environment
    // and drops when the call ends.
    let got = run({ () in
        let _ = kept.peek();
        moved
    });
    println("moved \(got)");
    println("after call \(drops)");
    if drops != 2 { return 1; }

    let e = counter(3);
    println("escaping \(runInt(e)) \(e())");
    drops = 0;
    callThenDrop();
    println("after drop \(drops)");
    0
}
