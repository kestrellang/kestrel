// test: diagnostics
// stdlib: true

module Main

func findFirst(target: std.num.Int64) -> std.result.Optional[std.num.Int64] {
    for i in std.core.Range[std.num.Int64](0, 100) {
        if i == target {
            return .Some(i)
        }
    }
    .None
}
