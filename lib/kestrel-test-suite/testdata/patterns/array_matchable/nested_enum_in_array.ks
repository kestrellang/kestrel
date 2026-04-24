// test: diagnostics
// stdlib: true

module Test

import std.result.Optional

func firstSome(arr: [Optional[lang.i64]]) -> lang.i64 {
    match arr {
        [.Some(x), ..] => x,
        _ => 0
    }
}
