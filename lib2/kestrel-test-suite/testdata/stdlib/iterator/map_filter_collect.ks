// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(5);

            // Test map
            let doubled = arr.iter().map({ (x) in x * 2 }).collect();
            if doubled.count != 5 { return 1 }
            if doubled(unchecked: 0) != 2 { return 2 }
            if doubled(unchecked: 4) != 10 { return 3 }

            // Test filter
            let evens = arr.iter().filter({ (x) in x % 2 == 0 }).collect();
            if evens.count != 2 { return 4 }
            if evens(unchecked: 0) != 2 { return 5 }
            if evens(unchecked: 1) != 4 { return 6 }

            // Test map + filter chain
            let result = arr.iter().filter({ (x) in x > 2 }).map({ (x) in x * 10 }).collect();
            if result.count != 3 { return 7 }
            if result(unchecked: 0) != 30 { return 8 }

            0
        }
