// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test isEmpty on empty deque
            var d = std.collections.Deque[std.numeric.Int64]();
            if d.isEmpty == false { return 1 }
            if d.count != 0 { return 2 }

            // Test pushBack and count
            d.pushBack(10);
            d.pushBack(20);
            d.pushBack(30);
            if d.count != 3 { return 3 }
            if d.isEmpty { return 4 }

            // Test popFront returns elements in FIFO order
            let f1 = d.popFront();
            if f1.unwrap() != 10 { return 5 }
            if d.count != 2 { return 6 }

            let f2 = d.popFront();
            if f2.unwrap() != 20 { return 7 }

            let f3 = d.popFront();
            if f3.unwrap() != 30 { return 8 }

            // popFront on empty
            let empty1 = d.popFront();
            if empty1.isSome() { return 9 }

            // Test pushFront
            d.pushFront(3);
            d.pushFront(2);
            d.pushFront(1);
            // Deque should be [1, 2, 3] front-to-back
            if d.count != 3 { return 10 }
            if d.popFront().unwrap() != 1 { return 11 }
            if d.popFront().unwrap() != 2 { return 12 }
            if d.popFront().unwrap() != 3 { return 13 }

            // Test popBack
            d.pushBack(10);
            d.pushBack(20);
            d.pushBack(30);
            let b1 = d.popBack();
            if b1.unwrap() != 30 { return 14 }
            if d.count != 2 { return 15 }

            let b2 = d.popBack();
            if b2.unwrap() != 20 { return 16 }

            let b3 = d.popBack();
            if b3.unwrap() != 10 { return 17 }

            // popBack on empty
            let empty2 = d.popBack();
            if empty2.isSome() { return 18 }

            // Mixed pushFront/pushBack
            d.pushBack(2);
            d.pushFront(1);
            d.pushBack(3);
            d.pushFront(0);
            // Deque should be [0, 1, 2, 3]
            if d.count != 4 { return 19 }
            if d.popFront().unwrap() != 0 { return 20 }
            if d.popBack().unwrap() != 3 { return 21 }
            if d.popFront().unwrap() != 1 { return 22 }
            if d.popBack().unwrap() != 2 { return 23 }
            if d.isEmpty == false { return 24 }

            // Test clear
            d.pushBack(1);
            d.pushBack(2);
            d.pushBack(3);
            d.clear();
            if d.count != 0 { return 25 }
            if d.isEmpty == false { return 26 }

            // Can push after clear
            d.pushBack(42);
            if d.popFront().unwrap() != 42 { return 27 }

            0
        }
