// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(3);
            arr.append(1);
            arr.append(4);
            arr.append(1);
            arr.append(5);

            // Test min
            let minVal = arr.iter().min();
            if minVal.isNone() { return 1 }
            if minVal.unwrap() != 1 { return 2 }

            // Test max
            let maxVal = arr.iter().max();
            if maxVal.isNone() { return 3 }
            if maxVal.unwrap() != 5 { return 4 }

            // Test sorted
            let sorted = arr.iter().sorted();
            if sorted.count != 5 { return 5 }
            if sorted(unchecked: 0) != 1 { return 6 }
            if sorted(unchecked: 4) != 5 { return 7 }

            // Test sum
            let sum = arr.iter().sum();
            if sum != 14 { return 8 }

            // Test product
            let product = [1, 2, 3].iter().product();
            if product != 6 { return 9 }

            0
        }
