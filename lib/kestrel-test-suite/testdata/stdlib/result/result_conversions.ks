// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(42);
            let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(99);

            // Test ok() on Ok - returns Some(value)
            let okOpt = ok.ok();
            if okOpt.isNone() { return 1 }
            if okOpt.unwrap() != 42 { return 2 }

            // Test ok() on Err - returns None
            let errOpt = err.ok();
            if errOpt.isSome() { return 3 }

            // Test err() on Err - returns Some(error)
            let errVal = err.err();
            if errVal.isNone() { return 4 }
            if errVal.unwrap() != 99 { return 5 }

            // Test err() on Ok - returns None
            let okErr = ok.err();
            if okErr.isSome() { return 6 }

            0
        }
