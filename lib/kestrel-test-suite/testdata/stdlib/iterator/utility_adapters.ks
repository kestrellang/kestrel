// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);

            // Test stepBy
            let everyOther: std.collections.Array[std.numeric.Int64] = [0, 1, 2, 3, 4, 5, 6].iter().stepBy(2).collect();
            if everyOther.count != 4 { return 1 }
            if everyOther(unchecked: 1) != 2 { return 2 }

            // Test scan (running sum)
            let running: std.collections.Array[std.numeric.Int64] = arr.iter().scan(from: 0, by: { (acc, x) in acc + x }).collect();
            if running.count != 3 { return 3 }
            if running(unchecked: 0) != 1 { return 4 }
            if running(unchecked: 2) != 6 { return 5 }

            // Test firstIndex
            let pos = arr.iter().firstIndex(where: { (x) in x == 2 });
            if pos.isNone() { return 6 }
            if pos.unwrap() != 1 { return 7 }

            // Test contains
            if arr.iter().contains(2) == false { return 8 }
            if arr.iter().contains(10) { return 9 }

            0
        }
