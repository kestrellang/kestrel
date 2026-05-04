// test: diagnostics
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(5);

            // Test count
            if arr.iter().count() != 5 { return 1 }
            if arr.iter().filter(matching: { (x) in x % 2 == 0 }).count() != 2 { return 2 }

            // Test fold (sum)
            let sum = arr.iter().fold(from: 0, combining: { (acc, x) in acc + x });
            if sum != 15 { return 3 }

            // Test any
            if arr.iter().any(matching: { (x) in x > 10 }) { return 4 }
            if arr.iter().any(matching: { (x) in x == 3 }) == false { return 5 }

            // Test all
            if arr.iter().all(matching: { (x) in x < 10 }) == false { return 6 }
            if arr.iter().all(matching: { (x) in x % 2 == 0 }) { return 7 }

            // Test position
            let found = arr.iter().firstIndex(matching: { (x) in x > 3 });
            if found.isNone() { return 8 }
            if found.unwrap() != 3 { return 9 }

            // Test nth
            let third = arr.iter().nth(2);
            if third.isNone() { return 10 }
            if third.unwrap() != 3 { return 11 }

            // Test first and last
            if arr.iter().first().unwrap() != 1 { return 12 }
            if arr.iter().last().unwrap() != 5 { return 13 }

            // Test forEach
            var total: std.numeric.Int64 = 0;
            arr.iter().forEach({ (x) in total = total + x }); // ERROR: cannot assign to captured variable
            if total != 15 { return 14 }

            0
        }
