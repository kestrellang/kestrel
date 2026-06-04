// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test empty init
            var d1 = std.collections.Deque[std.numeric.Int64]();
            if d1.count != 0 { return 1 }
            if d1.isEmpty == false { return 2 }

            // Test capacity init - starts empty but has reserved space
            var d2 = std.collections.Deque[std.numeric.Int64](capacity: 16);
            if d2.count != 0 { return 3 }
            if d2.isEmpty == false { return 4 }
            if d2.capacity < 16 { return 5 }

            // Can push into capacity-reserved deque
            d2.pushBack(10);
            d2.pushFront(5);
            if d2.count != 2 { return 6 }
            if d2.popFront().unwrap() != 5 { return 7 }
            if d2.popFront().unwrap() != 10 { return 8 }

            // Test capacity: 0 is equivalent to empty init
            var d3 = std.collections.Deque[std.numeric.Int64](capacity: 0);
            if d3.count != 0 { return 9 }
            d3.pushBack(1);
            if d3.popFront().unwrap() != 1 { return 10 }

            // Test from-iterable init using an array
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);
            arr.append(40);
            arr.append(50);
            let d4 = std.collections.Deque[std.numeric.Int64](from: arr);
            if d4.count != 5 { return 11 }

            // Elements should be in the same order as the source
            var d4m = d4.clone();
            if d4m.popFront().unwrap() != 10 { return 12 }
            if d4m.popFront().unwrap() != 20 { return 13 }
            if d4m.popFront().unwrap() != 30 { return 14 }
            if d4m.popFront().unwrap() != 40 { return 15 }
            if d4m.popFront().unwrap() != 50 { return 16 }

            // Test from-iterable with single element
            var single = std.collections.Array[std.numeric.Int64]();
            single.append(99);
            let d5 = std.collections.Deque[std.numeric.Int64](from: single);
            if d5.count != 1 { return 17 }

            // Test from-iterable with empty source
            let emptyArr = std.collections.Array[std.numeric.Int64]();
            let d6 = std.collections.Deque[std.numeric.Int64](from: emptyArr);
            if d6.count != 0 { return 18 }
            if d6.isEmpty == false { return 19 }

            // Test reserveCapacity
            var d7 = std.collections.Deque[std.numeric.Int64]();
            d7.reserveCapacity(minimumCapacity: 100);
            if d7.count != 0 { return 20 }
            d7.pushBack(7);
            if d7.popFront().unwrap() != 7 { return 21 }

            0
        }
