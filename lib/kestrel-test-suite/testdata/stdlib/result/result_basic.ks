// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(42);
            let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(99);

            // Test isOk
            if ok.isOk() == false { return 1 }
            if err.isOk() { return 2 }

            // Test isErr
            if ok.isErr() { return 3 }
            if err.isErr() == false { return 4 }

            // Test unwrap on Ok
            if ok.unwrap() != 42 { return 5 }

            // Test unwrapOr on Ok (returns contained value)
            if ok.unwrapOr(0) != 42 { return 6 }

            // Test unwrapOr on Err (returns default)
            if err.unwrapOr(0) != 0 { return 7 }

            // Test unwrapErr on Err
            if err.unwrapErr() != 99 { return 8 }

            0
        }
