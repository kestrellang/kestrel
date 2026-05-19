// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.core.(ControlFlow, Tryable)
import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Handle: not Copyable {
    var id: Int64

    deinit {
        deinit_count = deinit_count + 1;
    }
}

enum HandleResult: Tryable {
    case Ok(Handle)
    case Err(Int64)

    type Output = Handle
    type Residual = Int64

    public func tryExtract() -> ControlFlow[Handle, Int64] {
        match self {
            .Ok(handle) => .Continue(handle),
            .Err(code) => .Break(code)
        }
    }
}

func makeHandle() -> HandleResult {
    .Ok(Handle(id: 1))
}

func consume(consuming h: Handle) {}

func run() -> Result[Int64, Int64] {
    let h = try makeHandle();
    if deinit_count != 0 { return .Err(deinit_count); }

    consume(h);
    if deinit_count != 1 { return .Err(deinit_count); }

    .Ok(0)
}

func main() -> lang.i64 {
    match run() {
        .Ok(_) => if deinit_count == 1 { 0 } else { 2 },
        .Err(_) => 1
    }
}
