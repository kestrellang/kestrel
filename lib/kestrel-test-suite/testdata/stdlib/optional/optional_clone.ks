// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            let none: std.result.Optional[std.num.Int64] = .None;

            // Clone of Some
            let clonedSome = some.clone();
            if clonedSome.isNone() { return 1 }
            if clonedSome.unwrap() != 42 { return 2 }

            // Clone of None
            let clonedNone = none.clone();
            if clonedNone.isSome() { return 3 }

            0
        }
