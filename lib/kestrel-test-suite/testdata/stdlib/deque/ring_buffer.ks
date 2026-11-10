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
            d.append(1);
            d.append(2);
            d.append(3);
            d.append(4);
            if d.count != 4 { return 1 }

            // Pop two from the front -- head advances into the buffer
            if d.popFirst().unwrap() != 1 { return 2 }
            if d.popFirst().unwrap() != 2 { return 3 }
            // Deque is now [3, 4] with head somewhere in the middle

            // Push two more to wrap the tail around
            d.append(5);
            d.append(6);
            // Deque is [3, 4, 5, 6] -- tail has wrapped
            if d.count != 4 { return 4 }

            // Verify all elements via subscript (tests wrapped random access)
            if d(0) != 3 { return 5 }
            if d(1) != 4 { return 6 }
            if d(2) != 5 { return 7 }
            if d(3) != 6 { return 8 }

            // Pop all and verify order
            if d.popFirst().unwrap() != 3 { return 9 }
            if d.popFirst().unwrap() != 4 { return 10 }
            if d.popFirst().unwrap() != 5 { return 11 }
            if d.popFirst().unwrap() != 6 { return 12 }
            if d.isEmpty == false { return 13 }

            // Test pushFront wrap-around: head wraps backward past index 0
            var d2 = std.collections.Deque[std.numeric.Int64](capacity: 4);
            d2.append(10);
            d2.append(20);
            // Pop from front to move head forward
             d2.popFirst();
             d2.popFirst();
            // Now head is at index 2 (or similar), deque is empty.

            // pushFront wraps backward from the current head
            d2.prepend(100);
            d2.prepend(200);
            d2.prepend(300);
            // Deque is [300, 200, 100]
            if d2.count != 3 { return 14 }
            if d2(0) != 300 { return 15 }
            if d2(1) != 200 { return 16 }
            if d2(2) != 100 { return 17 }

            // Mixed front/back with wrap-around
            var d3 = std.collections.Deque[std.numeric.Int64](capacity: 4);
            // Push/pop cycle to advance head
            d3.append(0);
             d3.popFirst();
            d3.append(0);
             d3.popFirst();
            d3.append(0);
             d3.popFirst();
            // Head is now near the end of the buffer

            d3.append(1);
            d3.append(2);
            d3.prepend(0);
            // Deque is [0, 1, 2] with head wrapped around
            if d3.count != 3 { return 18 }
            if d3.popFirst().unwrap() != 0 { return 19 }
            if d3.popFirst().unwrap() != 1 { return 20 }
            if d3.popFirst().unwrap() != 2 { return 21 }

            // Stress test: many push/pop cycles to ensure stable behavior
            // after multiple wrap-arounds
            var d4 = std.collections.Deque[std.numeric.Int64](capacity: 4);
            for i in 0..<100 {
                d4.append(i);
                let val = d4.popFirst().unwrap();
                if val != i { return 22 }
            }
            if d4.isEmpty == false { return 23 }

            // Multi-element wrap-around stress: keep 2 elements in the deque
            // while cycling through
            var d5 = std.collections.Deque[std.numeric.Int64](capacity: 4);
            d5.append(0);
            d5.append(1);
            for i in 2..<50 {
                d5.append(i);
                let val = d5.popFirst().unwrap();
                if val != i - 2 { return 24 }
            }
            // Should have 2 elements left: 48, 49
            if d5.count != 2 { return 25 }
            if d5.popFirst().unwrap() != 48 { return 26 }
            if d5.popFirst().unwrap() != 49 { return 27 }

            // Test subscript set across wrapped boundary
            var d6 = std.collections.Deque[std.numeric.Int64](capacity: 4);
            d6.append(0);
            d6.append(0);
             d6.popFirst();
             d6.popFirst();
            // Head is advanced; push 4 elements to fill and wrap
            d6.append(1);
            d6.append(2);
            d6.append(3);
            d6.append(4);
            // Modify via subscript
            d6(0) = 10;
            d6(1) = 20;
            d6(2) = 30;
            d6(3) = 40;
            if d6.popFirst().unwrap() != 10 { return 28 }
            if d6.popFirst().unwrap() != 20 { return 29 }
            if d6.popFirst().unwrap() != 30 { return 30 }
            if d6.popFirst().unwrap() != 40 { return 31 }

            0
        }
