// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let _ = dict.insert(3, 30);

            // sumValues() returns sum of all values
            let total = dict.sumValues();
            if total != 60 { return 1 }

            // sumValues on empty dictionary returns default (0 for Int64)
            let emptyDict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let emptySum = emptyDict.sumValues();
            if emptySum != 0 { return 2 }

            // sumValues on single entry
            var singleDict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = singleDict.insert(1, 42);
            if singleDict.sumValues() != 42 { return 3 }

            // sumValues after mutation
            let _ = dict.insert(4, 40);
            if dict.sumValues() != 100 { return 4 }

            0
        }
