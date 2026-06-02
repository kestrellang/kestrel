// test: execution
// stdlib: true

// Regression: value promotion (`T` -> `Result[T,E]` / `Optional[T]`) at a
// return/tail site whose value is a `var` LOCAL. `lower_expr_for_return`'s
// var-local fast path (`load [take]`) returned the raw scalar WITHOUT applying
// the recorded promotion, so the unwrapped value was returned where a wrapped
// enum was expected — read as a pointer and dereferenced (segfault). The
// existing value_promotion_runtime_return test only covered literal tails,
// which take the non-var path, so this gap slipped through.
module Test

enum E { case Bad(std.text.String) }

// Throwing fn (-> Result[Int64, E]) whose tail is a `var` local.
func sumDigits(n: std.numeric.Int64) -> std.numeric.Int64 throws E {
    var r: std.numeric.Int64 = 0;
    var i: std.numeric.Int64 = 0;
    while i < n {
        if r > 1000000 { throw E.Bad("overflow"); }
        r = r * 10 + i;
        i = i + 1;
    }
    r
}

// Optional-returning fn whose tail is a `var` local.
func optVar(n: std.numeric.Int64) -> std.result.Optional[std.numeric.Int64] {
    var acc: std.numeric.Int64 = n;
    acc = acc + 1;
    acc
}

func main() -> lang.i64 {
    match sumDigits(4) { .Ok(v) => { if v != 123 { return 1; } }, .Err(_) => return 2 };
    match optVar(41) { some 42 => {}, _ => return 3 };
    0
}
