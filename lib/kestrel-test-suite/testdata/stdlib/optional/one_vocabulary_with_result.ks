// test: execution
// stdlib: true

module Test

import std.core.(Bool)

func half(x: Int64) -> Int64? {
    if x % 2 == 0 { .Some(x / 2) } else { .None }
}

func halfOrErr(x: Int64) -> Result[Int64, String] {
    if x % 2 == 0 { .Ok(x / 2) } else { .Err("odd") }
}

// Optional and Result share one chaining vocabulary (andValue / andThen /
// orValue / orElse), and their discriminator checks are properties.
@main
func main() -> lang.i64 {
    let present: Int64? = .Some(8);
    let absent: Int64? = .None;
    if not present.isSome { return 1 }
    if not absent.isNone { return 2 }

    if present.andThen(half).unwrap() != 4 { return 3 }
    if present.andValue(Optional[Int64].Some(1)).unwrap() != 1 { return 4 }
    if absent.orValue(Optional[Int64].Some(7)).unwrap() != 7 { return 5 }
    if present.orValue(Optional[Int64].Some(7)).unwrap() != 8 { return 6 }

    let ok: Result[Int64, String] = .Ok(8);
    let err: Result[Int64, String] = .Err("bad");
    if not ok.isOk { return 7 }
    if not err.isErr { return 8 }
    if ok.andThen(halfOrErr).unwrap() != 4 { return 9 }
    if err.orValue(ok).unwrap() != 8 { return 10 }
    if err.expectErr("should fail") != "bad" { return 11 }
    0
}
