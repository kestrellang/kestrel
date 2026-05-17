// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(10);
            let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(5);
            let other_ok: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(20);
            let other_err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(99);

            // Test andValue - Ok and Ok = second Ok
            let andOkOk = ok.andValue(other_ok);
            if andOkOk.unwrap() != 20 { return 1 }

            // Test andValue - Ok and Err = Err
            let andOkErr = ok.andValue(other_err);
            if andOkErr.isOk() { return 2 }
            if andOkErr.unwrapErr() != 99 { return 3 }

            // Test andValue - Err and Ok = Err (original error)
            let andErrOk = err.andValue(other_ok);
            if andErrOk.isOk() { return 4 }
            if andErrOk.unwrapErr() != 5 { return 5 }

            // Test andThen - Ok with transform
            let andThenOk = ok.andThen[std.numeric.Int64]({ (x) in .Ok(x * 3) });
            if andThenOk.unwrap() != 30 { return 6 }

            // Test andThen - Err passes through
            let andThenErr = err.andThen[std.numeric.Int64]({ (x) in .Ok(x * 3) });
            if andThenErr.isOk() { return 7 }
            if andThenErr.unwrapErr() != 5 { return 8 }

            // Test orValue - Ok returns self
            let orOk = ok.orValue(other_ok);
            if orOk.unwrap() != 10 { return 9 }

            // Test orValue - Err returns other
            let orErr = err.orValue(other_ok);
            if orErr.unwrap() != 20 { return 10 }

            // Test orElse - Ok returns self
            let orElseOk: std.result.Result[std.numeric.Int64, std.numeric.Int64] = ok.orElse({ (e) in .Ok(e + 100) });
            if orElseOk.unwrap() != 10 { return 11 }

            // Test orElse - Err calls alternative
            let orElseErr: std.result.Result[std.numeric.Int64, std.numeric.Int64] = err.orElse({ (e) in .Ok(e + 100) });
            if orElseErr.unwrap() != 105 { return 12 }

            // Test orElse - Err returning new Err
            let orElseNewErr: std.result.Result[std.numeric.Int64, std.numeric.Int64] = err.orElse({ (e) in .Err(e * 10) });
            if orElseNewErr.isOk() { return 13 }
            if orElseNewErr.unwrapErr() != 50 { return 14 }

            0
        }
