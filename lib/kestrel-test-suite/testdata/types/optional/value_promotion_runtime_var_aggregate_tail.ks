// test: execution
// stdlib: true

// Regression: value promotion (`T` -> `Result[T,E]` / `Optional[T]`) of a
// heap-backed AGGREGATE (`Array`/`String`) at a `throws`/optional tail produced
// an empty/dangling result. `apply_promotion` passed the value to
// `FromValue.from(value: T)` with the **Borrow** convention and then dropped the
// owned original — but `from` takes its argument *by value* (`.Ok(value)` moves
// it in). For an @owned aggregate the payload was wrapped from a borrow and the
// real value was freed, yielding `count == 0`. Scalars masked the bug: a borrow
// of an Int is bitwise-copied into the wrapper, so the dropped original didn't
// matter. (`value_promotion_runtime_var_tail.ks` only covers scalars.)
// Fix: `apply_promotion` consumes the value (Consuming convention).
//
// This is why talon-sqlite's `queryOnDb` returned 0 rows even though rows were
// read: it returns a `var results: Array[R]` from a `throws` function.
module Test

enum E { case Bad }

// `throws` (-> Result[Array, E]) with a `var` Array tail (promoted).
func buildArray(n: std.numeric.Int64) -> std.collections.Array[std.numeric.Int64] throws E {
    var results = std.collections.Array[std.numeric.Int64]();
    var i: std.numeric.Int64 = 0;
    while i < n {
        results.append(i * 10);
        i = i + 1;
    };
    results
}

// Optional-returning with a `var` String tail (promoted).
func buildString() -> std.result.Optional[std.text.String] {
    var s = std.text.String();
    s.append("ab");
    s.append("cd");
    s
}

func main() -> lang.i64 {
    match buildArray(3) {
        .Ok(arr) => {
            if arr.count != 3 { return 1; };
            if arr(0) != 0 { return 2; };
            if arr(2) != 20 { return 3; }
        },
        .Err(_) => return 4
    };
    match buildString() {
        some s => { if s.byteCount != 4 { return 5; } },
        null => return 6
    };
    0
}
