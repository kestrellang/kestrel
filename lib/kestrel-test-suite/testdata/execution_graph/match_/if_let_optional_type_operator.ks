// test: diagnostics
// stdlib: true

module Main

func unwrap(opt: std.numeric.Int64?) -> std.numeric.Int64 {
    if let .Some(v) = opt {
        v
    } else {
        0
    }
}
