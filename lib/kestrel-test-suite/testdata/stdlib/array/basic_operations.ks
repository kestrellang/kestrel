// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();

            // Test isEmpty on empty array
            if arr.isEmpty == false { return 1 }

            // Test append and count
            arr.append(10);
            arr.append(20);
            arr.append(30);
            if arr.count != 3 { return 2 }
            if arr.isEmpty { return 3 }

            // Test first and last
            if arr.first().unwrap() != 10 { return 4 }
            if arr.last().unwrap() != 30 { return 5 }

            // Test pop
            let popped = arr.pop();
            if popped.unwrap() != 30 { return 6 }
            if arr.count != 2 { return 7 }

            0
        }
