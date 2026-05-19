// test: execution
// stdlib: true
// expect-exit: 0

// Regression: try inside a loop desugars into a match whose arm bindings
// ($try_value) were function-scoped. On the next iteration the overwrite-drop
// fired *before* the new value was assigned, destroying a value that shares
// the same identity as the new one (e.g. a reused fd).

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Handle: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

func make_handle(id id: Int64) -> Result[Handle, Int64] {
    .Ok(Handle(id: id))
}

func use_handle(consuming h: Handle) -> Int64 {
    h.id
}

func run() -> Result[Int64, Int64] {
    var i: Int64 = 0;
    var sum: Int64 = 0;
    while i < 3 {
        var h = try make_handle(id: i);
        sum = sum + use_handle(h);
        i = i + 1;
    }
    .Ok(sum)
}

func main() -> lang.i64 {
    match run() {
        .Ok(sum) => {
            // sum should be 0+1+2 = 3
            if sum != 3 { return 1; }
            // Each iteration creates one Handle consumed by use_handle.
            // The try intermediates should also be cleaned up — exactly
            // 3 deinits total (one per iteration, from try's internal locals).
            if deinit_count < 3 { return 2; }
            0
        },
        .Err(_) => { return 3; }
    }
}
