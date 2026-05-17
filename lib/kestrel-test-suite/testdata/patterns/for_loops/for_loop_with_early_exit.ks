// test: diagnostics
// stdlib: true

module Main

func findFirst(target: std.numeric.Int64) -> std.result.Optional[std.numeric.Int64] {
    for i in std.core.Range[std.numeric.Int64](0, 100) {
        if i == target {
            return .Some(i)
        }
    }
    .None
}
