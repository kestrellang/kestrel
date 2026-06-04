// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test empty init
            var h1 = std.collections.Heap[std.numeric.Int64]();
            if h1.count != 0 { return 1 }
            if h1.isEmpty == false { return 2 }

            // Test capacity init - starts empty but has reserved space
            var h2 = std.collections.Heap[std.numeric.Int64](capacity: 16);
            if h2.count != 0 { return 3 }
            if h2.isEmpty == false { return 4 }

            // Can push into capacity-reserved heap
            h2.push(10);
            h2.push(5);
            if h2.count != 2 { return 5 }
            if h2.peek().unwrap() != 5 { return 6 }

            // Test from-iterable init using an array
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(5);
            arr.append(3);
            arr.append(1);
            arr.append(4);
            arr.append(2);
            let h3 = std.collections.Heap[std.numeric.Int64](from: arr);
            if h3.count != 5 { return 7 }
            // Peek should return the minimum element
            if h3.peek().unwrap() != 1 { return 8 }

            // Test from-iterable with single element
            var single = std.collections.Array[std.numeric.Int64]();
            single.append(42);
            let h4 = std.collections.Heap[std.numeric.Int64](from: single);
            if h4.count != 1 { return 9 }
            if h4.peek().unwrap() != 42 { return 10 }

            // Test from-iterable with empty array
            let emptyArr = std.collections.Array[std.numeric.Int64]();
            let h5 = std.collections.Heap[std.numeric.Int64](from: emptyArr);
            if h5.count != 0 { return 11 }
            if h5.isEmpty == false { return 12 }

            // Test reserveCapacity
            var h6 = std.collections.Heap[std.numeric.Int64]();
            h6.reserveCapacity(minimumCapacity: 100);
            if h6.count != 0 { return 13 }
            // Push should still work normally
            h6.push(7);
            if h6.peek().unwrap() != 7 { return 14 }

            0
        }
