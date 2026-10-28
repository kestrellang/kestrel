// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Exercise ring-buffer wrap-around by alternating push/pop patterns
            // that force the head pointer to advance past the end of the buffer.

            // Start with a small capacity so wrap-around happens quickly.
            var d = std.collections.Deque[std.numeric.Int64](capacity: 4);

            // Fill to capacity: [1, 2, 3, 4]
            d.pushBack(1);
            d.pushBack(2);
            d.pushBack(3);
            d.pushBack(4);
            if d.count != 4 { return 1 }

            // Pop two from the front -- head advances into the buffer
            if d.popFront().unwrap() != 1 { return 2 }
            if d.popFront().unwrap() != 2 { return 3 }
            // Deque is now [3, 4] with head somewhere in the middle

            // Push two more to wrap the tail around
            d.pushBack(5);
            d.pushBack(6);
            // Deque is [3, 4, 5, 6] -- tail has wrapped
            if d.count != 4 { return 4 }

            // Verify all elements via subscript (tests wrapped random access)
            if d(0) != 3 { return 5 }
            if d(1) != 4 { return 6 }
            if d(2) != 5 { return 7 }
            if d(3) != 6 { return 8 }

            // Pop all and verify order
            if d.popFront().unwrap() != 3 { return 9 }
            if d.popFront().unwrap() != 4 { return 10 }
            if d.popFront().unwrap() != 5 { return 11 }
            if d.popFront().unwrap() != 6 { return 12 }
            if d.isEmpty == false { return 13 }

            // Test pushFront wrap-around: head wraps backward past index 0
            var d2 = std.collections.Deque[std.numeric.Int64](capacity: 4);
            d2.pushBack(10);
            d2.pushBack(20);
            // Pop from front to move head forward
             d2.popFront();
             d2.popFront();
            // Now head is at index 2 (or similar), deque is empty.

            // pushFront wraps backward from the current head
            d2.pushFront(100);
            d2.pushFront(200);
            d2.pushFront(300);
            // Deque is [300, 200, 100]
            if d2.count != 3 { return 14 }
            if d2(0) != 300 { return 15 }
            if d2(1) != 200 { return 16 }
            if d2(2) != 100 { return 17 }

            // Mixed front/back with wrap-around
            var d3 = std.collections.Deque[std.numeric.Int64](capacity: 4);
            // Push/pop cycle to advance head
            d3.pushBack(0);
             d3.popFront();
            d3.pushBack(0);
             d3.popFront();
            d3.pushBack(0);
             d3.popFront();
            // Head is now near the end of the buffer

            d3.pushBack(1);
            d3.pushBack(2);
            d3.pushFront(0);
            // Deque is [0, 1, 2] with head wrapped around
            if d3.count != 3 { return 18 }
            if d3.popFront().unwrap() != 0 { return 19 }
            if d3.popFront().unwrap() != 1 { return 20 }
            if d3.popFront().unwrap() != 2 { return 21 }

            // Stress test: many push/pop cycles to ensure stable behavior
            // after multiple wrap-arounds
            var d4 = std.collections.Deque[std.numeric.Int64](capacity: 4);
            for i in 0..<100 {
                d4.pushBack(i);
                let val = d4.popFront().unwrap();
                if val != i { return 22 }
            }
            if d4.isEmpty == false { return 23 }

            // Multi-element wrap-around stress: keep 2 elements in the deque
            // while cycling through
            var d5 = std.collections.Deque[std.numeric.Int64](capacity: 4);
            d5.pushBack(0);
            d5.pushBack(1);
            for i in 2..<50 {
                d5.pushBack(i);
                let val = d5.popFront().unwrap();
                if val != i - 2 { return 24 }
            }
            // Should have 2 elements left: 48, 49
            if d5.count != 2 { return 25 }
            if d5.popFront().unwrap() != 48 { return 26 }
            if d5.popFront().unwrap() != 49 { return 27 }

            // Test subscript set across wrapped boundary
            var d6 = std.collections.Deque[std.numeric.Int64](capacity: 4);
            d6.pushBack(0);
            d6.pushBack(0);
             d6.popFront();
             d6.popFront();
            // Head is advanced; push 4 elements to fill and wrap
            d6.pushBack(1);
            d6.pushBack(2);
            d6.pushBack(3);
            d6.pushBack(4);
            // Modify via subscript
            d6(0) = 10;
            d6(1) = 20;
            d6(2) = 30;
            d6(3) = 40;
            if d6.popFront().unwrap() != 10 { return 28 }
            if d6.popFront().unwrap() != 20 { return 29 }
            if d6.popFront().unwrap() != 30 { return 30 }
            if d6.popFront().unwrap() != 40 { return 31 }

            0
        }
