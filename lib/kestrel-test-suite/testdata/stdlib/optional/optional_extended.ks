// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            let none: std.result.Optional[std.num.Int64] = .None;

            // Test isSomeAnd - Some with true predicate
            if some.isSomeAnd({ (x) in x > 0 }) == false { return 1 }

            // Test isSomeAnd - Some with false predicate
            if some.isSomeAnd({ (x) in x < 0 }) { return 2 }

            // Test isSomeAnd - None always false
            if none.isSomeAnd({ (x) in x > 0 }) { return 3 }

            // Test expect on Some (should not panic, returns value)
            if some.expect("should have value") != 42 { return 4 }

            // Test unwrap(orElse:) on Some (should return contained value, not call closure)
            if some.unwrap(orElse: { () in 99 }) != 42 { return 5 }

            // Test unwrap(orElse:) on None (should call closure)
            if none.unwrap(orElse: { () in 99 }) != 99 { return 6 }

            // Test inspect on Some (returns self unchanged)
            // Note: cannot modify captured variables in closures, so just verify return value
            let inspected = some.inspect({ (x) in });
            if inspected.unwrap() != 42 { return 7 }

            // Test inspect on None (returns None)
            let inspectedNone = none.inspect({ (x) in });
            if inspectedNone.isSome() { return 8 }

            // Test xor - Some xor None = Some
            let xorResult1 = some.xor(.None);
            if xorResult1.unwrap() != 42 { return 9 }

            // Test xor - None xor Some = Some
            let xorResult2 = none.xor(.Some(99));
            if xorResult2.unwrap() != 99 { return 10 }

            // Test xor - Some xor Some = None
            let xorResult3 = some.xor(.Some(99));
            if xorResult3.isSome() { return 11 }

            // Test xor - None xor None = None
            let xorResult4 = none.xor(.None);
            if xorResult4.isSome() { return 12 }

            // Test zip - Some zip Some = Some tuple
            let zipped = some.zip(with: .Some(100));
            if zipped.isNone() { return 13 }
            let pair = zipped.unwrap();
            if pair.0 != 42 { return 14 }
            if pair.1 != 100 { return 15 }

            // Test zip - Some zip None = None
            let zippedNone: std.result.Optional[(std.num.Int64, std.num.Int64)] = some.zip(with: .None);
            if zippedNone.isSome() { return 16 }

            // Test zip - None zip Some = None
            let noneZipped: std.result.Optional[(std.num.Int64, std.num.Int64)] = none.zip(with: .Some(100));
            if noneZipped.isSome() { return 17 }

            0
        }
