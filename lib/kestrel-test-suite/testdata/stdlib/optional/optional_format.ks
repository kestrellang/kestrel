// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.numeric.Int64] = .Some(42);
            let none: std.result.Optional[std.numeric.Int64] = .None;

            // Format Some
            let someStr = some.format();
            if someStr.isEqual(to: "Some(42)") == false { return 1 }

            // Format None
            let noneStr = none.format();
            if noneStr.isEqual(to: "None") == false { return 2 }

            0
        }
