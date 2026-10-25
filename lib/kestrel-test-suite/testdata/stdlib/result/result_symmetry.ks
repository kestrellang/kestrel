// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let ok: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(42);
            let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(99);

            // isOkAnd(where:)
            if ok.isOkAnd(where: { (x) in x > 0 }) == false { return 1 }
            if ok.isOkAnd(where: { (x) in x < 0 }) { return 2 }
            if err.isOkAnd(where: { (x) in x > 0 }) { return 3 }

            // isErrAnd(where:)
            if err.isErrAnd(where: { (e) in e == 99 }) == false { return 4 }
            if err.isErrAnd(where: { (e) in e == 0 }) { return 5 }
            if ok.isErrAnd(where: { (e) in e == 99 }) { return 6 }

            // expect(message:) on Ok
            let okCopy: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(7);
            if okCopy.expect("must be ok") != 7 { return 7 }

            // inspect returns self unchanged
            let tapped = ok.inspect({ (x) in () });
            if tapped.isOk() == false { return 8 }
            if tapped.unwrap() != 42 { return 9 }

            // inspect on Err skips fn, passes Err through
            let errTapped = err.inspect({ (x) in () });
            if errTapped.isErr() == false { return 10 }

            // inspectErr returns self unchanged
            let err2: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(99);
            let errTapped2 = err2.inspectErr({ (e) in () });
            if errTapped2.unwrapErr() != 99 { return 11 }
            let ok2: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(5);
            let okTapped = ok2.inspectErr({ (e) in () });
            if okTapped.unwrap() != 5 { return 12 }

            // flatten
            let nestedOk: std.result.Result[std.result.Result[std.numeric.Int64, std.numeric.Int64], std.numeric.Int64] = .Ok(.Ok(42));
            let flatOk = nestedOk.flatten();
            if flatOk.unwrap() != 42 { return 13 }

            let nestedInnerErr: std.result.Result[std.result.Result[std.numeric.Int64, std.numeric.Int64], std.numeric.Int64] = .Ok(.Err(1));
            let flatInnerErr = nestedInnerErr.flatten();
            if flatInnerErr.unwrapErr() != 1 { return 14 }

            let nestedOuterErr: std.result.Result[std.result.Result[std.numeric.Int64, std.numeric.Int64], std.numeric.Int64] = .Err(2);
            let flatOuterErr = nestedOuterErr.flatten();
            if flatOuterErr.unwrapErr() != 2 { return 15 }

            0
        }
