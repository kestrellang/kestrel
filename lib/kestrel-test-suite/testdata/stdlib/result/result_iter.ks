// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(42);
            let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(99);

            // iter on Ok - yields 1 element
            var okIter = ok.iter();
            let first = okIter.next();
            if first.isNone() { return 1 }
            if first.unwrap() != 42 { return 2 }

            // Second call returns None
            let second = okIter.next();
            if second.isSome() { return 3 }

            // iter on Err - yields 0 elements
            var errIter = err.iter();
            let errFirst = errIter.next();
            if errFirst.isSome() { return 4 }

            0
        }
