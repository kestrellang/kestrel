// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let someOpt: std.result.Optional[std.numeric.Int64] = .Some(42);
            let none: std.result.Optional[std.numeric.Int64] = .None;

            // Test okOr on Some - returns Ok(value)
            let okResult: std.result.Result[std.numeric.Int64, std.numeric.Int64] = someOpt.okOr( 99);
            if okResult.isOk() == false { return 1 }
            if okResult.unwrap() != 42 { return 2 }

            // Test okOr on None - returns Err(error)
            let errResult: std.result.Result[std.numeric.Int64, std.numeric.Int64] = none.okOr( 99);
            if errResult.isErr() == false { return 3 }
            if errResult.unwrapErr() != 99 { return 4 }

            // Test okOrElse on Some - returns Ok(value), no call
            let okResult2: std.result.Result[std.numeric.Int64, std.numeric.Int64] = someOpt.okOrElse({ () in 99 });
            if okResult2.isOk() == false { return 5 }
            if okResult2.unwrap() != 42 { return 6 }

            // Test okOrElse on None - calls function, returns Err
            let errResult2: std.result.Result[std.numeric.Int64, std.numeric.Int64] = none.okOrElse({ () in 99 });
            if errResult2.isErr() == false { return 7 }
            if errResult2.unwrapErr() != 99 { return 8 }

            0
        }
