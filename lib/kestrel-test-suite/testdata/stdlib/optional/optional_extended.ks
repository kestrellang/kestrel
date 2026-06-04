// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let someOpt: std.result.Optional[std.numeric.Int64] = .Some(42);
            let none: std.result.Optional[std.numeric.Int64] = .None;

            // Test isSomeAnd - Some with true predicate
            if someOpt.isSomeAnd({ (x) in x > 0 }) == false { return 1 }

            // Test isSomeAnd - Some with false predicate
            if someOpt.isSomeAnd({ (x) in x < 0 }) { return 2 }

            // Test isSomeAnd - None always false
            if none.isSomeAnd({ (x) in x > 0 }) { return 3 }

            // Test expect on Some (should not panic, returns value)
            if someOpt.expect("should have value") != 42 { return 4 }

            // Test unwrap(orElse:) on Some (should return contained value, not call closure)
            if someOpt.unwrap(orElse: { () in 99 }) != 42 { return 5 }

            // Test unwrap(orElse:) on None (should call closure)
            if none.unwrap(orElse: { () in 99 }) != 99 { return 6 }

            // Test inspect on Some (returns self unchanged)
            // Note: cannot modify captured variables in closures, so just verify return value
            let inspected = someOpt.inspect({ (x) in });
            if inspected.unwrap() != 42 { return 7 }

            // Test inspect on None (returns None)
            let inspectedNone = none.inspect({ (x) in });
            if inspectedNone.isSome() { return 8 }

            // Test xor - Some xor None = Some
            let xorResult1 = someOpt.xor(.None);
            if xorResult1.unwrap() != 42 { return 9 }

            // Test xor - None xor Some = Some
            let xorResult2 = none.xor(.Some(99));
            if xorResult2.unwrap() != 99 { return 10 }

            // Test xor - Some xor Some = None
            let xorResult3 = someOpt.xor(.Some(99));
            if xorResult3.isSome() { return 11 }

            // Test xor - None xor None = None
            let xorResult4 = none.xor(.None);
            if xorResult4.isSome() { return 12 }

            // Test zip - Some zip Some = Some tuple
            let zipped = someOpt.zip(with: .Some(100));
            if zipped.isNone() { return 13 }
            let pair = zipped.unwrap();
            if pair.0 != 42 { return 14 }
            if pair.1 != 100 { return 15 }

            // Test zip - Some zip None = None
            let zippedNone: std.result.Optional[(std.numeric.Int64, std.numeric.Int64)] = someOpt.zip(with: .None);
            if zippedNone.isSome() { return 16 }

            // Test zip - None zip Some = None
            let noneZipped: std.result.Optional[(std.numeric.Int64, std.numeric.Int64)] = none.zip(with: .Some(100));
            if noneZipped.isSome() { return 17 }

            0
        }
