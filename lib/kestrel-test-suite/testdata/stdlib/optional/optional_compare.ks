// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let none: std.result.Optional[std.numeric.Int64] = .None;
            let some1: std.result.Optional[std.numeric.Int64] = .Some(1);
            let some2: std.result.Optional[std.numeric.Int64] = .Some(2);
            let some1b: std.result.Optional[std.numeric.Int64] = .Some(1);

            // None < Some(x) for any x
            if none.compare(some1) != std.core.Ordering.Less { return 1 }

            // Some(x) > None
            if some1.compare(none) != std.core.Ordering.Greater { return 2 }

            // None == None
            if none.compare(none) != std.core.Ordering.Equal { return 3 }

            // Some(1) < Some(2)
            if some1.compare(some2) != std.core.Ordering.Less { return 4 }

            // Some(2) > Some(1)
            if some2.compare(some1) != std.core.Ordering.Greater { return 5 }

            // Some(1) == Some(1)
            if some1.compare(some1b) != std.core.Ordering.Equal { return 6 }

            0
        }
