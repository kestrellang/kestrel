// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let a: std.result.Optional[std.numeric.Int64] = .Some(42);
            let b: std.result.Optional[std.numeric.Int64] = .Some(42);
            let c: std.result.Optional[std.numeric.Int64] = .Some(99);
            let none1: std.result.Optional[std.numeric.Int64] = .None;
            let none2: std.result.Optional[std.numeric.Int64] = .None;

            // Some(42) == Some(42)
            if a.isEqual(to: b) == false { return 1 }

            // Some(42) != Some(99)
            if a.isEqual(to: c) { return 2 }

            // Some(42) != None
            if a.isEqual(to: none1) { return 3 }

            // None != Some(42)
            if none1.isEqual(to: a) { return 4 }

            // None == None
            if none1.isEqual(to: none2) == false { return 5 }

            0
        }
