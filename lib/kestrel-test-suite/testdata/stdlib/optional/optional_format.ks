// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let someOpt: std.result.Optional[std.numeric.Int64] = .Some(42);
            let none: std.result.Optional[std.numeric.Int64] = .None;

            // Format Some
            let someStr = someOpt.formatted();
            if someStr.isEqual(to: "Some(42)") == false { return 1 }

            // Format None
            let noneStr = none.formatted();
            if noneStr.isEqual(to: "None") == false { return 2 }

            0
        }
