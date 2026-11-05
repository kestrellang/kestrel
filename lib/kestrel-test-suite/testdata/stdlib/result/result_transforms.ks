// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let ok: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(10);
            let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(5);

            // Test map on Ok - transforms value
            let mapped = ok.map({ (x) in x * 2 });
            if mapped.unwrap() != 20 { return 1 }

            // Test map on Err - passes through error
            let mappedErr = err.map({ (x) in x * 2 });
            if mappedErr.isOk { return 2 }
            if mappedErr.unwrapErr() != 5 { return 3 }

            // Test andThen on Ok
            let flatMapped = ok.andThen[std.numeric.Int64]({ (x) in .Ok(x + 1) });
            if flatMapped.unwrap() != 11 { return 4 }

            // Test andThen on Ok returning Err
            let flatMappedErr: std.result.Result[std.numeric.Int64, std.numeric.Int64] = ok.andThen[std.numeric.Int64]({ (x) in .Err(x) });
            if flatMappedErr.isOk { return 5 }
            if flatMappedErr.unwrapErr() != 10 { return 6 }

            // Test andThen on Err - passes through error
            let flatMappedOnErr = err.andThen[std.numeric.Int64]({ (x) in .Ok(x + 1) });
            if flatMappedOnErr.isOk { return 7 }
            if flatMappedOnErr.unwrapErr() != 5 { return 8 }

            // Test mapErr on Err - transforms error
            let mappedErrVal = err.mapErr({ (e) in e * 10 });
            if mappedErrVal.isOk { return 9 }
            if mappedErrVal.unwrapErr() != 50 { return 10 }

            // Test mapErr on Ok - passes through value
            let mappedErrOk = ok.mapErr({ (e) in e * 10 });
            if mappedErrOk.unwrap() != 10 { return 11 }

            // Test orElse on Err - transforms error with Result-returning fn
            let flatMappedErrVal: std.result.Result[std.numeric.Int64, std.numeric.Int64] = err.orElse[std.numeric.Int64]({ (e) in .Err(e * 10) });
            if flatMappedErrVal.isOk { return 12 }
            if flatMappedErrVal.unwrapErr() != 50 { return 13 }

            // Test orElse on Err recovering to Ok
            let recovered = err.orElse[std.numeric.Int64]({ (e) in .Ok(e + 100) });
            if recovered.isErr { return 14 }
            if recovered.unwrap() != 105 { return 15 }

            // Test orElse on Ok - passes through value
            let flatMappedErrOk = ok.orElse[std.numeric.Int64]({ (e) in .Err(e * 10) });
            if flatMappedErrOk.unwrap() != 10 { return 16 }

            0
        }
