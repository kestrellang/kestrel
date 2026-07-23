// test: execution
// stdlib: true

module Test

        // filter and isSomeAnd take their predicate under the `where:`
        // label (NAMING_CONVENTIONS predicate rule).
        @main
        func main() -> lang.i64 {
            let someOpt: std.result.Optional[std.numeric.Int64] = .Some(10);
            let none: std.result.Optional[std.numeric.Int64] = .None;

            if someOpt.isSomeAnd(where: { (x) in x > 5 }) == false { return 1 }
            if someOpt.isSomeAnd(where: { (x) in x > 50 }) { return 2 }
            if none.isSomeAnd(where: { (x) in x > 5 }) { return 3 }

            let kept = someOpt.filter(where: { (x) in x > 5 });
            if kept.unwrap() != 10 { return 4 }

            let someOpt2: std.result.Optional[std.numeric.Int64] = .Some(10);
            let dropped = someOpt2.filter(where: { (x) in x > 50 });
            if dropped.isSome() { return 5 }

            let noneFiltered = none.filter(where: { (x) in x > 5 });
            if noneFiltered.isSome() { return 6 }

            0
        }
