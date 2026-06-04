// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var s = std.collections.Set[std.numeric.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);
            let _ = s.insert(3);
            let _ = s.insert(4);
            let _ = s.insert(5);

            // Test filter(where:)
            let evens = s.filter(where: { (x) in x % 2 == 0 });
            if evens.count != 2 { return 1 }
            if evens.contains(2) == false { return 2 }
            if evens.contains(4) == false { return 3 }
            if evens.contains(1) { return 4 }

            // Test map()
            let doubled = s.map({ (x) in x * 2 });
            if doubled.count != 5 { return 5 }
            if doubled.contains(2) == false { return 6 }
            if doubled.contains(10) == false { return 7 }

            // Test map() with collisions - duplicates removed
            let modThree = s.map({ (x) in x % 3 });
            // 1%3=1, 2%3=2, 3%3=0, 4%3=1, 5%3=2 -> {0, 1, 2}
            if modThree.count != 3 { return 8 }

            // Test toArray()
            let arr = s.toArray();
            if arr.count != 5 { return 9 }

            // Test sorted() - returns array (note: sort not yet implemented, returns unsorted)
            let sorted = s.sorted();
            if sorted.count != 5 { return 10 }

            // Test min() and max()
            let minVal = s.min();
            if minVal.isNone() { return 11 }
            if minVal.unwrap() != 1 { return 12 }

            let maxVal = s.max();
            if maxVal.isNone() { return 13 }
            if maxVal.unwrap() != 5 { return 14 }

            // Test filter on original set unchanged
            if s.count != 5 { return 15 }

            0
        }
