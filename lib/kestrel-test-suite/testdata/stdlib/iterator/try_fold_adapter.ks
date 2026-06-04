// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // tryFold: fold where combine returns Result
            // Successful fold - all Ok. The error type `E` is unused here,
            // so it needs an explicit annotation (see try_fold_unconstrained_error_type.ks
            // for the diagnostic we emit when the annotation is missing).
            let result: std.result.Result[std.numeric.Int64, std.numeric.Int64] =
                [1, 2, 3, 4].iter().tryFold(from: 0, by: { (acc, x) in
                    .Ok(acc + x)
                });
            match result {
                .Ok(v) => { if v != 10 { return 1 } },
                .Err(_) => { return 2 }
            }

            // tryFold with early exit on error
            let earlyExit = [1, 2, 3, 4, 5].iter().tryFold(from: 0, by: { (acc, x) in
                if acc > 3 {
                    let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(acc);
                    err
                } else {
                    .Ok(acc + x)
                }
            });
            match earlyExit {
                .Ok(_) => { return 3 },
                .Err(e) => { if e != 6 { return 4 } }
            }

            // tryFold on empty iterator returns Ok(initial)
            let empty = std.collections.Array[std.numeric.Int64]();
            let emptyResult: std.result.Result[std.numeric.Int64, std.numeric.Int64] = empty.iter().tryFold(from: 42, by: { (acc, x) in
                .Ok(acc + x)
            });
            match emptyResult {
                .Ok(v) => { if v != 42 { return 5 } },
                .Err(_) => { return 6 }
            }

            // tryFold that errors on first element
            let firstErr = [1, 2, 3].iter().tryFold(from: 0, by: { (acc, x) in
                let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(-1);
                err
            });
            match firstErr {
                .Ok(_) => { return 7 },
                .Err(e) => { if e != -1 { return 8 } }
            }

            0
        }
