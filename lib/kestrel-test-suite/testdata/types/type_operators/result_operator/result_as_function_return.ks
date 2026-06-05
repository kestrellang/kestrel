// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

struct ParseError {}

func parse(valid: std.core.Bool) -> std.numeric.Int64 throws ParseError {
    if valid {
        .Ok(42)
    } else {
        .Err(ParseError())
    }
}

func main() -> lang.i64 {
    let ok = parse(true);
    let err = parse(false);
     println(ok.unwrap());
     println(err.isErr());
    0
}
