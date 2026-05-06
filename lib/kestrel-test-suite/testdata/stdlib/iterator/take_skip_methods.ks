// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(5);

            // Test take
            let first3 = arr.iter().take(3).collect();
            if first3.count != 3 { return 1 }
            if first3(unchecked: 2) != 3 { return 2 }

            // Test skip
            let last3 = arr.iter().skip(2).collect();
            if last3.count != 3 { return 3 }
            if last3(unchecked: 0) != 3 { return 4 }

            // Test takeWhile
            let lessThan4 = arr.iter().takeWhile(where: { (x) in x < 4 }).collect();
            if lessThan4.count != 3 { return 5 }
            if lessThan4(unchecked: 2) != 3 { return 6 }

            // Test skipWhile
            let from4 = arr.iter().skipWhile(where: { (x) in x < 4 }).collect();
            if from4.count != 2 { return 7 }
            if from4(unchecked: 0) != 4 { return 8 }

            0
        }
