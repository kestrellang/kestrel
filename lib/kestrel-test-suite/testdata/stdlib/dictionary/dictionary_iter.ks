// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let _ = dict.insert(3, 30);

            // iter() - iterate over all key-value pairs
            var count: std.numeric.Int64 = 0;
            var keySum: std.numeric.Int64 = 0;
            var valSum: std.numeric.Int64 = 0;
            var iter = dict.iter();
            while let .Some(pair) = iter.next() {
                count = count + 1;
                keySum = keySum + pair.0;
                valSum = valSum + pair.1;
            }
            if count != 3 { return 1 }
            if keySum != 6 { return 2 }
            if valSum != 60 { return 3 }

            // iter() on empty dictionary
            let emptyDict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            var emptyIter = emptyDict.iter();
            let first = emptyIter.next();
            if first.isSome() { return 4 }

            // iter() - verify all expected keys are present
            var found1 = false;
            var found2 = false;
            var found3 = false;
            var iter2 = dict.iter();
            while let .Some(pair) = iter2.next() {
                if pair.0 == 1 and pair.1 == 10 { found1 = true }
                if pair.0 == 2 and pair.1 == 20 { found2 = true }
                if pair.0 == 3 and pair.1 == 30 { found3 = true }
            }
            if found1 == false { return 5 }
            if found2 == false { return 6 }
            if found3 == false { return 7 }

            0
        }
