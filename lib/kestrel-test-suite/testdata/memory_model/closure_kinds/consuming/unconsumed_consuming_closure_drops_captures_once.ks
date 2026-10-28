// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md, Copy and Drop: for `consuming`, "the sole owner
// drops the environment, dropping any captures not already moved out by the
// one call." A closure that is never called still releases its captured
// resource exactly once when it goes out of scope.
module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Res: not Copyable {
    var id: Int64
    func peek() -> Int64 { self.id }
    deinit { deinit_count = deinit_count + 1; }
}

func scope() {
    let r = Res(id: 4);
    // `r` moves into the owning environment; `f` is never called, so the drop
    // of `f` at scope exit is what destroys the Res.
    let f: consuming () -> () = { () in let _ = r.peek(); };
    let _ = f;
}

@main
func main() -> lang.i64 {
    scope();
    if deinit_count != 1 { return 1; }
    0
}
