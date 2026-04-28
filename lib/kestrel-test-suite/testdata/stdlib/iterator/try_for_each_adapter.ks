// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // tryForEach: forEach where action returns Result
            // All succeed
            let result = [1, 2, 3].iter().tryForEach({ (x) in
                let ok: std.result.Result[(), std.numeric.Int64] = .Ok(());
                ok
            });
            match result {
                .Ok(_) => {},
                .Err(_) => { return 1 }
            }

            // tryForEach with early exit on error
            // Error when we encounter a value > 3
            let earlyExit = [1, 2, 3, 4, 5].iter().tryForEach({ (x) in
                if x > 3 {
                    let err: std.result.Result[(), std.numeric.Int64] = .Err(x);
                    err
                } else {
                    .Ok(())
                }
            });
            match earlyExit {
                .Ok(_) => { return 2 },
                .Err(e) => { if e != 4 { return 3 } }
            }

            // tryForEach on empty iterator returns Ok
            let empty = std.collections.Array[std.numeric.Int64]();
            let emptyResult = empty.iter().tryForEach({ (x) in
                let err: std.result.Result[(), std.numeric.Int64] = .Err(x);
                err
            });
            match emptyResult {
                .Ok(_) => {},
                .Err(_) => { return 4 }
            }

            0
        }
