// test: diagnostics
// stdlib: true

module Main

func unwrap(opt: std.num.Int64?) -> std.num.Int64 {
    if let .Some(v) = opt {
        v
    } else {
        0
    }
}
