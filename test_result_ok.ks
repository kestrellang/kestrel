module test

import std.result.(Result)

func test() -> Result[Int64, Int64] {
    Result.ok(value: 42)
}
