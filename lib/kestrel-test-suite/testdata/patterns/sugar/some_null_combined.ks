// test: execution
// stdlib: true

module Test

func describe(opt: std.result.Optional[std.numeric.Int64]) -> lang.i64 {
    match opt {
        null => 0,
        some _ => 1
    }
}

func main() -> lang.i64 {
    match describe(.Some(7)) {
        1 => match describe(.None) {
            0 => match describe(.Some(35)) {
                1 => 0,
                _ => 3
            },
            _ => 2
        },
        _ => 1
    }
}
