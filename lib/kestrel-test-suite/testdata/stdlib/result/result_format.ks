// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let ok: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(42);
            let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(99);

            // Format Ok
            let okStr = ok.formatted();
            if okStr.isEqual(to: "Ok(42)") == false { return 1 }

            // Format Err
            let errStr = err.formatted();
            if errStr.isEqual(to: "Err(99)") == false { return 2 }

            0
        }
