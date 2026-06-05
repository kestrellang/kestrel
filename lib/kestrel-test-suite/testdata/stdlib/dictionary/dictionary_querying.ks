// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
             dict.insert(1, 10);
             dict.insert(2, 20);
             dict.insert(3, 30);

            // Test keys - iterate and count
            var keyCount: std.numeric.Int64 = 0;
            var keyIter = dict.keys.iter();
            while let .Some(_) = keyIter.next() {
                keyCount = keyCount + 1;
            }
            if keyCount != 3 { return 1 }

            // Test values - iterate and sum
            var valSum: std.numeric.Int64 = 0;
            var valIter = dict.values.iter();
            while let .Some(v) = valIter.next() {
                valSum = valSum + v;
            }
            if valSum != 60 { return 2 }

            // Test contains(where:) - true case
            let hasLargeValue = dict.contains(where: { (k, v) in v > 25 });
            if hasLargeValue == false { return 3 }

            // Test contains(where:) - false case
            let hasHugeValue = dict.contains(where: { (k, v) in v > 100 });
            if hasHugeValue { return 4 }

            // Test all(where:) - true case
            let allPositive = dict.all(where: { (k, v) in v > 0 });
            if allPositive == false { return 5 }

            // Test all(where:) - false case
            let allBig = dict.all(where: { (k, v) in v > 15 });
            if allBig { return 6 }

            // Test any(where:)
            let anyTwenty = dict.any(where: { (k, v) in v == 20 });
            if anyTwenty == false { return 7 }

            let anyHundred = dict.any(where: { (k, v) in v == 100 });
            if anyHundred { return 8 }

            // Test countWhere()
            let countAbove15 = dict.countItems(where: { (k, v) in v > 15 });
            if countAbove15 != 2 { return 9 }

            // Test first(where:)
            let found = dict.first(where: { (k, v) in v == 20 });
            if found.isNone() { return 10 }
            let (fk, fv) = found.unwrap();
            if fk != 2 { return 11 }
            if fv != 20 { return 12 }

            // Test first(where:) - not found
            let notFound = dict.first(where: { (k, v) in v == 999 });
            if notFound.isSome() { return 13 }

            // Test all(where:) on empty dictionary - vacuous truth
            var emptyDict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let vacuousAll = emptyDict.all(where: { (k, v) in false });
            if vacuousAll == false { return 14 }

            0
        }
