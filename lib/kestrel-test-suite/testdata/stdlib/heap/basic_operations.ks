// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test isEmpty on empty heap
            var h = std.collections.Heap[std.numeric.Int64]();
            if h.isEmpty == false { return 1 }
            if h.count != 0 { return 2 }

            // Test push and count
            h.push(30);
            h.push(10);
            h.push(20);
            if h.count != 3 { return 3 }
            if h.isEmpty { return 4 }

            // Test peek returns minimum without removing
            if h.peek().unwrap() != 10 { return 5 }
            if h.count != 3 { return 6 }

            // Test pop returns minimum and removes it
            let first = h.pop();
            if first.unwrap() != 10 { return 7 }
            if h.count != 2 { return 8 }

            // Next pop should return next smallest
            let second = h.pop();
            if second.unwrap() != 20 { return 9 }
            if h.count != 1 { return 10 }

            // Last pop
            let third = h.pop();
            if third.unwrap() != 30 { return 11 }
            if h.count != 0 { return 12 }
            if h.isEmpty == false { return 13 }

            // Pop on empty returns None
            let empty = h.pop();
            if empty.isSome() { return 14 }

            // Peek on empty returns None
            let emptyPeek = h.peek();
            if emptyPeek.isSome() { return 15 }

            // Test clear
            h.push(5);
            h.push(3);
            h.push(7);
            h.clear();
            if h.count != 0 { return 16 }
            if h.isEmpty == false { return 17 }

            // Can push again after clear
            h.push(42);
            if h.peek().unwrap() != 42 { return 18 }

            0
        }
