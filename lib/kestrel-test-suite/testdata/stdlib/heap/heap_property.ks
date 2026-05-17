// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Verify min-heap: popping always yields ascending order.

            // Already-sorted input
            var h1 = std.collections.Heap[std.numeric.Int64]();
            h1.push(1);
            h1.push(2);
            h1.push(3);
            h1.push(4);
            h1.push(5);
            if h1.pop().unwrap() != 1 { return 1 }
            if h1.pop().unwrap() != 2 { return 2 }
            if h1.pop().unwrap() != 3 { return 3 }
            if h1.pop().unwrap() != 4 { return 4 }
            if h1.pop().unwrap() != 5 { return 5 }

            // Reverse-sorted input
            var h2 = std.collections.Heap[std.numeric.Int64]();
            h2.push(5);
            h2.push(4);
            h2.push(3);
            h2.push(2);
            h2.push(1);
            if h2.pop().unwrap() != 1 { return 6 }
            if h2.pop().unwrap() != 2 { return 7 }
            if h2.pop().unwrap() != 3 { return 8 }
            if h2.pop().unwrap() != 4 { return 9 }
            if h2.pop().unwrap() != 5 { return 10 }

            // Random-ish order
            var h3 = std.collections.Heap[std.numeric.Int64]();
            h3.push(42);
            h3.push(7);
            h3.push(99);
            h3.push(1);
            h3.push(55);
            h3.push(23);
            h3.push(3);
            if h3.pop().unwrap() != 1 { return 11 }
            if h3.pop().unwrap() != 3 { return 12 }
            if h3.pop().unwrap() != 7 { return 13 }
            if h3.pop().unwrap() != 23 { return 14 }
            if h3.pop().unwrap() != 42 { return 15 }
            if h3.pop().unwrap() != 55 { return 16 }
            if h3.pop().unwrap() != 99 { return 17 }

            // Duplicate values
            var h4 = std.collections.Heap[std.numeric.Int64]();
            h4.push(3);
            h4.push(1);
            h4.push(3);
            h4.push(1);
            h4.push(2);
            if h4.pop().unwrap() != 1 { return 18 }
            if h4.pop().unwrap() != 1 { return 19 }
            if h4.pop().unwrap() != 2 { return 20 }
            if h4.pop().unwrap() != 3 { return 21 }
            if h4.pop().unwrap() != 3 { return 22 }

            // Interleaved push/pop: peek always shows current min
            var h5 = std.collections.Heap[std.numeric.Int64]();
            h5.push(10);
            if h5.peek().unwrap() != 10 { return 23 }
            h5.push(5);
            if h5.peek().unwrap() != 5 { return 24 }
            h5.push(15);
            if h5.peek().unwrap() != 5 { return 25 }
            let _ = h5.pop(); // removes 5
            if h5.peek().unwrap() != 10 { return 26 }
            h5.push(3);
            if h5.peek().unwrap() != 3 { return 27 }
            let _ = h5.pop(); // removes 3
            if h5.peek().unwrap() != 10 { return 28 }
            let _ = h5.pop(); // removes 10
            if h5.peek().unwrap() != 15 { return 29 }

            // Single element
            var h6 = std.collections.Heap[std.numeric.Int64]();
            h6.push(99);
            if h6.pop().unwrap() != 99 { return 30 }
            if h6.pop().isSome() { return 31 }

            // From-iterable preserves heap property
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(8);
            arr.append(3);
            arr.append(10);
            arr.append(1);
            arr.append(6);
            arr.append(2);
            arr.append(9);
            arr.append(4);
            arr.append(7);
            arr.append(5);
            var h7 = std.collections.Heap[std.numeric.Int64](from: arr);
            // Pop all and verify sorted order
            var prev: std.numeric.Int64 = 0;
            for i in 0..<10 {
                let val = h7.pop().unwrap();
                if val < prev { return 32 }
                prev = val
            }
            if h7.pop().isSome() { return 33 }

            0
        }
