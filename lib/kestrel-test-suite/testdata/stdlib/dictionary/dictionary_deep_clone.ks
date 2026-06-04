// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = dict.insert(1, 100);
            let _ = dict.insert(2, 200);
            let _ = dict.insert(3, 300);

            // deepClone creates a fully independent copy
            let cloned = dict.deepClone();
            if cloned.count != 3 { return 1 }
            if cloned(1).unwrap() != 100 { return 2 }
            if cloned(2).unwrap() != 200 { return 3 }
            if cloned(3).unwrap() != 300 { return 4 }

            // Mutating original should not affect clone
            let _ = dict.insert(4, 400);
            if dict.count != 4 { return 5 }
            if cloned.count != 3 { return 6 }

            // deepClone on empty dictionary
            let emptyDict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let emptyClone = emptyDict.deepClone();
            if emptyClone.count != 0 { return 7 }

            0
        }
