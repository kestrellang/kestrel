// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test take - takes value out, leaving None
            var opt1: std.result.Optional[std.numeric.Int64] = .Some(42);
            let taken = opt1.take();
            if taken.unwrap() != 42 { return 1 }
            if opt1.isSome() { return 2 }

            // Test take on None
            var opt2: std.result.Optional[std.numeric.Int64] = .None;
            let takenNone = opt2.take();
            if takenNone.isSome() { return 3 }
            if opt2.isSome() { return 4 }

            // Test replace - replaces value, returns old
            var opt3: std.result.Optional[std.numeric.Int64] = .Some(10);
            let old = opt3.replace(20);
            if old.unwrap() != 10 { return 5 }
            if opt3.unwrap() != 20 { return 6 }

            // Test replace on None - returns None, sets to Some
            var opt4: std.result.Optional[std.numeric.Int64] = .None;
            let oldNone = opt4.replace(50);
            if oldNone.isSome() { return 7 }
            if opt4.unwrap() != 50 { return 8 }

            // Test take(matching:) - predicate true, takes value
            var opt5: std.result.Optional[std.numeric.Int64] = .Some(42);
            let takenIf = opt5.take(matching:{ (x) in x > 0 });
            if takenIf.unwrap() != 42 { return 9 }
            if opt5.isSome() { return 10 }

            // Test take(matching:) - predicate false, leaves value
            var opt6: std.result.Optional[std.numeric.Int64] = .Some(42);
            let notTaken = opt6.take(matching:{ (x) in x < 0 });
            if notTaken.isSome() { return 11 }
            if opt6.isNone() { return 12 }
            if opt6.unwrap() != 42 { return 13 }

            // Test take(matching:) on None
            var opt7: std.result.Optional[std.numeric.Int64] = .None;
            let takeIfNone = opt7.take(matching:{ (x) in x > 0 });
            if takeIfNone.isSome() { return 14 }

            0
        }
