// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test first and last on empty deque
            var d = std.collections.Deque[std.numeric.Int64]();
            if d.first().isSome() { return 1 }
            if d.last().isSome() { return 2 }

            // Test first and last with one element
            d.pushBack(42);
            if d.first().unwrap() != 42 { return 3 }
            if d.last().unwrap() != 42 { return 4 }

            // Test first and last with multiple elements
            d.pushBack(99);
            d.pushFront(7);
            // Deque is [7, 42, 99]
            if d.first().unwrap() != 7 { return 5 }
            if d.last().unwrap() != 99 { return 6 }

            // Test subscript get
            if d(0) != 7 { return 7 }
            if d(1) != 42 { return 8 }
            if d(2) != 99 { return 9 }

            // Test subscript set
            d(0) = 100;
            if d(0) != 100 { return 10 }
            if d.first().unwrap() != 100 { return 11 }

            d(2) = 200;
            if d(2) != 200 { return 12 }
            if d.last().unwrap() != 200 { return 13 }

            d(1) = 150;
            if d(1) != 150 { return 14 }

            // Verify all elements after set
            // Deque is now [100, 150, 200]
            if d.popFront().unwrap() != 100 { return 15 }
            if d.popFront().unwrap() != 150 { return 16 }
            if d.popFront().unwrap() != 200 { return 17 }

            // Test first/last update after pop operations
            d.pushBack(1);
            d.pushBack(2);
            d.pushBack(3);
            d.pushBack(4);
            // Deque is [1, 2, 3, 4]
            let _ = d.popFront(); // removes 1
            if d.first().unwrap() != 2 { return 18 }
            if d.last().unwrap() != 4 { return 19 }
            let _ = d.popBack(); // removes 4
            if d.first().unwrap() != 2 { return 20 }
            if d.last().unwrap() != 3 { return 21 }

            // Subscript access on elements pushed from front
            var d2 = std.collections.Deque[std.numeric.Int64]();
            d2.pushFront(3);
            d2.pushFront(2);
            d2.pushFront(1);
            // Logical order is [1, 2, 3]
            if d2(0) != 1 { return 22 }
            if d2(1) != 2 { return 23 }
            if d2(2) != 3 { return 24 }

            0
        }
