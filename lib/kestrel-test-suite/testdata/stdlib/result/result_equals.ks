// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let ok1: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(42);
            let ok2: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(42);
            let ok3: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(99);
            let err1: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(5);
            let err2: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(5);
            let err3: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(10);

            // Ok(42) == Ok(42)
            if ok1.isEqual(to: ok2) == false { return 1 }

            // Ok(42) != Ok(99)
            if ok1.isEqual(to: ok3) { return 2 }

            // Err(5) == Err(5)
            if err1.isEqual(to: err2) == false { return 3 }

            // Err(5) != Err(10)
            if err1.isEqual(to: err3) { return 4 }

            // Ok(42) != Err(5)
            if ok1.isEqual(to: err1) { return 5 }

            // Err(5) != Ok(42)
            if err1.isEqual(to: ok1) { return 6 }

            0
        }
