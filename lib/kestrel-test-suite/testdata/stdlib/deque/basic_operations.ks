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
            d.append(10);
            d.append(20);
            d.append(30);
            if d.count != 3 { return 3 }
            if d.isEmpty { return 4 }

            // Test popFront returns elements in FIFO order
            let f1 = d.popFirst();
            if f1.unwrap() != 10 { return 5 }
            if d.count != 2 { return 6 }

            let f2 = d.popFirst();
            if f2.unwrap() != 20 { return 7 }

            let f3 = d.popFirst();
            if f3.unwrap() != 30 { return 8 }

            // popFront on empty
            let empty1 = d.popFirst();
            if empty1.isSome { return 9 }

            // Test pushFront
            d.prepend(3);
            d.prepend(2);
            d.prepend(1);
            // Deque should be [1, 2, 3] front-to-back
            if d.count != 3 { return 10 }
            if d.popFirst().unwrap() != 1 { return 11 }
            if d.popFirst().unwrap() != 2 { return 12 }
            if d.popFirst().unwrap() != 3 { return 13 }

            // Test popBack
            d.append(10);
            d.append(20);
            d.append(30);
            let b1 = d.popLast();
            if b1.unwrap() != 30 { return 14 }
            if d.count != 2 { return 15 }

            let b2 = d.popLast();
            if b2.unwrap() != 20 { return 16 }

            let b3 = d.popLast();
            if b3.unwrap() != 10 { return 17 }

            // popBack on empty
            let empty2 = d.popLast();
            if empty2.isSome { return 18 }

            // Mixed pushFront/pushBack
            d.append(2);
            d.prepend(1);
            d.append(3);
            d.prepend(0);
            // Deque should be [0, 1, 2, 3]
            if d.count != 4 { return 19 }
            if d.popFirst().unwrap() != 0 { return 20 }
            if d.popLast().unwrap() != 3 { return 21 }
            if d.popFirst().unwrap() != 1 { return 22 }
            if d.popLast().unwrap() != 2 { return 23 }
            if d.isEmpty == false { return 24 }

            // Test clear
            d.append(1);
            d.append(2);
            d.append(3);
            d.clear();
            if d.count != 0 { return 25 }
            if d.isEmpty == false { return 26 }

            // Can push after clear
            d.append(42);
            if d.popFirst().unwrap() != 42 { return 27 }

            0
        }
