// test: execution
// stdlib: true

module Test

        var fallbackCalls: std.numeric.Int64 = 0;

        func fallback() -> std.numeric.Int64 {
            fallbackCalls = fallbackCalls + 1;
            return 7;
        }

        @main
        func main() -> lang.i64 {
            let ok: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(42);
            let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(99);

            // ?? on Ok yields the value; the default thunk must not run
            if (ok ?? fallback()) != 42 { return 1 }
            if fallbackCalls != 0 { return 2 }

            // ?? on Err yields the default; the thunk runs exactly once
            if (err ?? fallback()) != 7 { return 3 }
            if fallbackCalls != 1 { return 4 }

            // direct coalesce call, no operator sugar
            let ok2: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(1);
            if ok2.coalesce({ 0 }) != 1 { return 5 }

            // non-Copyable (String) success payload moves out cleanly
            let okStr: std.result.Result[std.text.String, std.numeric.Int64] = .Ok("hello");
            if (okStr ?? "other") != "hello" { return 6 }
            let errStr: std.result.Result[std.text.String, std.numeric.Int64] = .Err(1);
            if (errStr ?? "other") != "other" { return 7 }

            return 0
        }
