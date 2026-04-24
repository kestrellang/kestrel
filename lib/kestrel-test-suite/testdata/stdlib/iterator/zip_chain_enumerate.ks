// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr1 = std.collections.Array[std.num.Int64]();
            arr1.append(1);
            arr1.append(2);
            arr1.append(3);

            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(10);
            arr2.append(20);
            arr2.append(30);

            // Test zip
            let zipped = arr1.iter().zip(arr2.iter()).collect();
            if zipped.count != 3 { return 1 }
            let (a, b) = zipped(unchecked: 0);
            if a != 1 { return 2 }
            if b != 10 { return 3 }

            // Test enumerate
            let enumerated = arr1.iter().enumerate().collect();
            if enumerated.count != 3 { return 4 }
            let (idx, val) = enumerated(unchecked: 1);
            if idx != 1 { return 5 }
            if val != 2 { return 6 }

            // Test chain
            let chained = arr1.iter().chain(arr2.iter()).collect();
            if chained.count != 6 { return 7 }
            if chained(unchecked: 3) != 10 { return 8 }

            0
        }
