// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            let none: std.result.Optional[std.num.Int64] = .None;

            // Format Some
            let someStr = some.format();
            if someStr.equals("Some(42)") == false { return 1 }

            // Format None
            let noneStr = none.format();
            if noneStr.equals("None") == false { return 2 }

            0
        }
