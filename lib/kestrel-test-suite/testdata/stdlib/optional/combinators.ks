// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let someOpt: std.result.Optional[std.numeric.Int64] = .Some(10);
            let none: std.result.Optional[std.numeric.Int64] = .None;
            let other: std.result.Optional[std.numeric.Int64] = .Some(20);

            // Test then (and combinator)
            let andResult = someOpt.then(other);
            if andResult.unwrap() != 20 { return 1 }
            let andNone = none.then(other);
            if andNone.isSome() { return 2 }

            // Test orElse (without closure capture to avoid codegen bug)
            let orResult: std.result.Optional[std.numeric.Int64] = none.orElse({ () in .Some(99) });
            if orResult.unwrap() != 99 { return 3 }
            let orSome: std.result.Optional[std.numeric.Int64] = someOpt.orElse({ () in .Some(99) });
            if orSome.unwrap() != 10 { return 4 }

            0
        }
