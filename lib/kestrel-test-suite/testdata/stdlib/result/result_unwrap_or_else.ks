// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let ok: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(42);
            let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(5);

            // unwrap(orElse:) on Ok - returns contained value, doesn't call function
            let okVal = ok.unwrap(orElse: { (e) in e * 100 });
            if okVal != 42 { return 1 }

            // unwrap(orElse:) on Err - calls function with error
            let errVal = err.unwrap(orElse: { (e) in e * 100 });
            if errVal != 500 { return 2 }

            // unwrap(orElse:) on Err with recovery to fixed value
            let recovered = err.unwrap(orElse: { (e) in 0 });
            if recovered != 0 { return 3 }

            0
        }
